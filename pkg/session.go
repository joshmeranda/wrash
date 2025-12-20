package wrash

import (
	"bufio"
	"fmt"
	"io"
	"log/slog"
	"os"
	"os/exec"
	"path/filepath"
	"runtime/debug"
	"strings"
	"unicode"

	prompt "github.com/joshmeranda/go-prompt"
	"github.com/joshmeranda/wrash/pkg/args"
	"github.com/samber/lo"
	"github.com/urfave/cli/v2"
)

const (
	runeset = "`~!@#$%^&*()-=+[{]}\\|;:'\",.<>/?_"

	completionsDir = "completions"
)

// todo: this should support changing cases as well as just whitespaces
func getNextBoundary(runeset string, text string) int {
	if text == "" {
		return 0
	}

	startIsBoundary := strings.ContainsRune(runeset, rune(text[0])) || unicode.IsSpace(rune(text[0]))
	var i int

	// todo: might need to handle non-english localizations (chinese, japanese, etc)
	for ; i < len(text); i++ {
		isBoundary := strings.ContainsRune(runeset, rune(text[i])) || unicode.IsSpace(rune(text[i]))

		if startIsBoundary != isBoundary {
			break
		}
	}

	return i
}

func goNextBoundary(buff *prompt.Buffer) {
	startPosition := buff.DisplayCursorPosition()
	text := buff.Text()
	boundary := getNextBoundary(runeset, text[startPosition:])
	buff.CursorRight(boundary)
}

func goPreviousBoundary(buff *prompt.Buffer) {
	startPosition := buff.DisplayCursorPosition()

	// todo: creating a new reversed string like this is pretty expensive, we probably want to update getNextBoundary to support a reverse mode
	// reverse text
	text := string(lo.Reverse([]rune(buff.Text())))

	startPosition = len(text) - startPosition

	boundary := getNextBoundary(runeset, text[startPosition:])
	buff.CursorLeft(boundary)
}

// todo: add session id
type Session struct {
	Base string

	stdout io.Writer
	stderr io.Writer
	stdin  io.Reader

	prompt      *prompt.Prompt
	interactive bool // useful for disable tty requirement for testing

	environ     map[string]string
	completions map[string]*commandCompleter
	configDir   string

	history          *history
	exitCalled       bool
	previousExitCode int
	apps             map[string]*cli.App
	isFrozen         bool // todo: is never used
}

func NewSession(base string, opts ...Option) (*Session, error) {
	session := &Session{
		Base: base,

		environ:     make(map[string]string),
		completions: make(map[string]*commandCompleter),

		interactive: true,

		stdout: os.Stdout,
		stderr: os.Stderr,
		stdin:  os.Stdin,
	}

	for _, opt := range opts {
		if err := opt(session); err != nil {
			return nil, fmt.Errorf("error applying option: %w", err)
		}
	}

	session.initBuiltins()

	if err := session.loadCompletions(); err != nil {
		return nil, fmt.Errorf("failed to load completions: %w", err)
	}

	if session.history == nil {
		session.history = NewHistory(base, io.Discard, make([]*Entry, 0)).(*history)
	}

	if session.interactive {
		session.prompt = prompt.New(session.executor, session.completer,
			prompt.OptionTitle("wrash"+base),
			prompt.OptionPrefix(base+" >"),
			prompt.OptionHistory(session.history),
			prompt.OptionLivePrefix(session.livePrefix),
			prompt.OptionSetExitCheckerOnInput(func(_ string, breakline bool) bool {
				return breakline && session.exitCalled
			}),
			prompt.OptionAddKeyBind(
				prompt.KeyBind{
					Key: prompt.ControlRight,
					Fn:  goNextBoundary,
				},
				prompt.KeyBind{
					Key: prompt.ControlLeft,
					Fn:  goPreviousBoundary,
				},
			),
		)
	}

	return session, nil
}

func (s *Session) runFile(path string) error {
	f, err := os.Open(path)
	if err != nil {
		return err
	}

	scanner := bufio.NewScanner(f)

	for n := 1; scanner.Scan(); n++ {
		line := strings.TrimSpace(scanner.Text())

		if line == "" {
			continue
		}

		cmd, err := args.Parse(line)
		if err != nil {
			return fmt.Errorf("could not parse line %d: %s", n, err)
		}

		expanded, err := cmd.Expand(func(key string) string {
			return s.environ[key]
		})
		if err != nil {
			return fmt.Errorf("could not expand line: %w", err)
		}

		app, found := s.apps[expanded[0]]
		if !found {
			return fmt.Errorf("found unknown builtin '%s' on line %d", expanded[0], n)
		}

		if err := app.Run(expanded); err != nil {
			return err
		}
	}

	return nil
}

func (s *Session) loadCompletions() error {
	dir := filepath.Join(s.configDir, completionsDir)

	entries, err := os.ReadDir(dir)
	if os.IsNotExist(err) {
		return nil
	} else if err != nil {
		slog.Error("error loading completions", "err", err)
		return nil
	}

	for _, entry := range entries {
		if entry.Type().IsRegular() {
			if err := s.runFile(filepath.Join(dir, entry.Name())); err != nil {
				return fmt.Errorf("failed to load '%s': %w", filepath.Join(dir, entry.Name()), err)
			}
		}
	}

	return nil
}

func (s *Session) executor(str string) {
	defer s.history.Clear()

	if strings.TrimSpace(str) == "" {
		return
	}

	cmd, err := args.Parse(s.Base + " " + str)
	if err != nil {
		fmt.Fprintf(s.stderr, "could not parse args: %s\n", err)
		return
	}

	expanded, err := cmd.Expand(func(key string) string {
		return s.environ[key]
	})
	if err != nil {
		fmt.Fprintf(s.stderr, "could not expand args: %s\n", err)
		return
	}

	s.previousExitCode = 0

	if isBuiltin(str) {
		expanded = expanded[1:]
		app, found := s.apps[expanded[0][2:]]
		if !found {
			fmt.Fprintf(s.stderr, "unknown builtin: %s\n", expanded[0])
			s.previousExitCode = 127
			return
		}

		if err := app.Run(expanded); err != nil {
			fmt.Fprintf(s.stderr, "could not run command: %s\n", err)
			s.previousExitCode = 127
		}
	} else {
		cmd := exec.Command(expanded[0], expanded[1:]...)
		cmd.Stdout = s.stdout
		cmd.Stderr = s.stderr
		cmd.Stdin = s.stdin

		if err := cmd.Run(); err != nil {
			switch err := err.(type) {
			case *exec.ExitError:
				s.previousExitCode = err.ExitCode()
			default:
				s.previousExitCode = 127
				fmt.Fprintf(s.stderr, "could not run command: %s\n", err)
			}
		}
	}
}

func (s *Session) livePrefix() (string, bool) {
	user := os.Getenv("USER")
	wd, err := os.Getwd()

	if err != nil {
		slog.Error("failed to get working dir", "err", err)
		wd = "%HOME%"
	}

	return fmt.Sprintf("[%s %s] %s > ", user, wd, s.Base), true
}

func (s *Session) completer(doc prompt.Document) []prompt.Suggest {
	var completer Completer

	completer, ok := s.completions[s.Base]

	if !ok {
		switch {
		case strings.HasPrefix(doc.TextBeforeCursor(), "!!"):
			completer = CompleterFunc(s.builtinsCompleter)

		default:
			completer = CompleterFunc(fileCompleter)
		}
	}

	suggestions := completer.Complete(doc)

	return suggestions
}

func (s *Session) Run() {
	defer func() {
		if err := recover(); err != nil {
			fmt.Println(fmt.Sprintf("%s", string(debug.Stack())))
			slog.Error("encountered panic",
				"err", err,
				attrKeyStacktrace, string(debug.Stack()),
			)
		}

		if err := s.history.Sync(); err != nil {
			slog.Error("could not sync history", "err", err)
		} else {
			slog.Info("synced history")
		}
	}()

	slog.Info("staring new Wrash session...",
		"interactive", s.interactive,
		"configDir", s.configDir,
		// todo: log history file
	)

	s.prompt.Run()
}

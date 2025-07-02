package wrash

import (
	"fmt"
	"io"
	"os"
	"os/exec"
	"strings"
	"unicode"

	prompt "github.com/joshmeranda/go-prompt"
	"github.com/joshmeranda/wrash/pkg/args"
	"github.com/samber/lo"
	"github.com/urfave/cli/v2"
)

const (
	runeset = "`~!@#$%^&*()-=+[{]}\\|;:'\",.<>/?_"
)

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

type Session struct {
	Base string

	stdout io.Writer
	stderr io.Writer
	stdin  io.Reader

	prompt      *prompt.Prompt
	interactive bool // useful for disable tty requirement for testing

	environ map[string]string

	history          *history
	exitCalled       bool
	previousExitCode int
	apps             map[string]*cli.App
	isFrozen         bool
}

func NewSession(base string, opts ...Option) (*Session, error) {
	session := &Session{
		Base: base,

		environ: make(map[string]string),

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

	if session.history == nil {
		session.history = NewHistory(base, io.Discard, make([]*Entry, 0)).(*history)
	}

	session.initBuiltins()

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
			fmt.Fprintf(s.stderr, "unknown command: %s\n", expanded[0])
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
	wd, _ := os.Getwd()
	return fmt.Sprintf("[%s %s] %s > ", user, wd, s.Base), true
}

func (s *Session) completer(doc prompt.Document) []prompt.Suggest {
	var suggestions []prompt.Suggest

	switch {
	case strings.HasPrefix(doc.TextBeforeCursor(), "!!"):
		suggestions = lo.Filter(lo.MapToSlice(s.apps, func(name string, app *cli.App) prompt.Suggest {
			return prompt.Suggest{
				Text:        "!!" + app.Name,
				Description: app.Description,
			}
		}), func(s prompt.Suggest, _ int) bool {
			return strings.HasPrefix(s.Text, doc.TextBeforeCursor())
		})
	default:
		suggestions = fileCompleter(doc)
	}

	return suggestions
}

func (s *Session) Run() {
	defer func() {
		if err := s.history.Sync(); err != nil {
			fmt.Fprintf(s.stderr, "could not sync history: %s\n", err)
		}
	}()

	s.prompt.Run()
}

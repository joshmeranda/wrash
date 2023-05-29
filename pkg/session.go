package wrash

import (
	"fmt"
	"io"
	"os"
	"os/exec"
	"strings"
	"unicode"

	"github.com/google/shlex"
	prompt "github.com/joshmeranda/go-prompt"
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
	text := buff.Text()

	// todo: creatinga new reversed string like this is pretty expensive, we probblay want to update getNextBoundary to support a reverse mode
	// reverse text
	runes := []rune(text)
	for i, j := 0, len(runes)-1; i < j; i, j = i+1, j-1 {
		runes[i], runes[j] = runes[j], runes[i]
	}

	text = string(runes)
	startPosition = len(text) - startPosition

	boundary := getNextBoundary(runeset, text[startPosition:])
	buff.CursorLeft(boundary)
}

type Session struct {
	Base string

	stdout io.Writer
	stderr io.Writer
	stdin  io.Reader

	prompt        *prompt.Prompt
	disablePrompt bool // useful for disable tty requirement for testing

	history          *history
	exitCalled       bool
	previousExitCode int
	apps             map[string]*cli.App
	isFrozen         bool
}

func NewSession(base string, opts ...Option) (*Session, error) {
	session := &Session{
		Base: base,

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
		session.history = NewHistory(session, make([]*Entry, 0)).(*history)
	}

	session.initApps()

	if !session.disablePrompt {
		// todo: OptionLivePrefix
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

func (s *Session) initApps() {
	// todo: might be worth using the stderr/out/in of the Session
	s.apps = make(map[string]*cli.App)

	s.apps["cd"] = &cli.App{
		Name:        "cd",
		Usage:       "cd [TARGET]",
		Description: "change the working directory of the shell",
		Flags:       []cli.Flag{},
		Action:      s.doCd,
	}

	s.apps["exit"] = &cli.App{
		Name:        "exit",
		Usage:       "exit [CODE]",
		Description: "exit the shell",
		Action:      s.doExit,
	}

	s.apps["help"] = &cli.App{
		Name:        "help",
		Usage:       "help",
		Description: "view help text",
		Action:      s.doHelp,
	}

	s.apps["history"] = &cli.App{
		Name:        "histroy",
		Usage:       "history [pattern]",
		Description: "view the history of the shell (pattern should not inclue the base command)",
		Action:      s.doHistory,
		Flags: []cli.Flag{
			&cli.IntFlag{
				Name:    "number",
				Aliases: []string{"n"},
				Usage:   "limit shown history entries to N (if N is 0, all entries will be shown)",
			},
			&cli.BoolFlag{
				Name:    "show",
				Aliases: []string{"s"},
				Usage:   "include the base command in the output",
			},
		},
	}
}

func (s *Session) executor(str string) {
	// todo: might need to defer a history reset
	if str == "" {
		return
	}

	s.previousExitCode = 0

	if isBuiltin(str) {
		args, err := shlex.Split(str[2:])
		if err != nil {
			fmt.Fprintf(s.stderr, "could not parse input: %s\n", err)
		}

		app, found := s.apps[args[0]]
		if !found {
			fmt.Fprintf(s.stderr, "unknown command: %s\n", args[0])
			s.previousExitCode = 127
			return
		}

		if err := app.Run(args); err != nil {
			fmt.Fprintf(s.stderr, "could not run command: %s\n", err)
			s.previousExitCode = 127
		}
	} else {
		args, err := shlex.Split(s.Base + " " + str)
		if err != nil {
			fmt.Printf("could not parse input: %s\n", err)
		}

		cmd := exec.Command(args[0], args[1:]...)
		cmd.Stdout = os.Stdout
		cmd.Stderr = os.Stderr
		cmd.Stdin = os.Stdin

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

// todo: load completions from a file
// todo: add completions for builtins
func (s *Session) completer(doc prompt.Document) []prompt.Suggest {
	return fileCompleter(doc)
}

func (s *Session) Run() {
	defer func() {
		if err := s.history.Sync(); err != nil {
			fmt.Fprintf(s.stderr, "could not sync history: %s\n", err)
		}
	}()

	s.prompt.Run()
}

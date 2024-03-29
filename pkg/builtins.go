package wrash

import (
	"fmt"
	"os"
	"regexp"
	"sort"
	"strconv"
	"strings"

	"github.com/samber/lo"
	"github.com/urfave/cli/v2"
)

// todo: we only need to specify each builtin's cli.Apps in, out, and err, or use the Sessions in, out, or err not both

func isBuiltin(s string) bool {
	return strings.HasPrefix(s, "!!")
}

func (s *Session) initBuiltins() {
	s.apps = make(map[string]*cli.App)

	s.apps["cd"] = &cli.App{
		Name:        "cd",
		Usage:       "cd [TARGET]",
		Description: "change the working directory of the shell",
		Flags:       []cli.Flag{},
		Action:      s.doCd,

		Reader:    s.stdin,
		Writer:    s.stdout,
		ErrWriter: s.stderr,
	}

	s.apps["exit"] = &cli.App{
		Name:        "exit",
		Usage:       "exit [CODE]",
		Description: "exit the shell",
		Action:      s.doExit,

		Reader:    s.stdin,
		Writer:    s.stdout,
		ErrWriter: s.stderr,
	}

	s.apps["help"] = &cli.App{
		Name:        "help",
		Usage:       "help",
		Description: "view help text",
		Action:      s.doHelp,

		Reader:    s.stdin,
		Writer:    s.stdout,
		ErrWriter: s.stderr,
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

		Reader:    s.stdin,
		Writer:    s.stdout,
		ErrWriter: s.stderr,
	}

	s.apps["env"] = &cli.App{
		Name:        "env",
		Usage:       "env",
		Description: "set or display environment variables for the current session",
		Action:      s.doEnv,
		Commands: []*cli.Command{
			{
				Name:        "set",
				Usage:       "env set [KEY [VALUE]]",
				Description: "set environment variables for the current session",
				Action:      s.doEnv,
			},
			{
				Name:        "show",
				Usage:       "env show",
				Description: "show environment variables for the current session",
				Action:      s.doEnv,
			},
		},

		DefaultCommand: "show",

		Reader:    s.stdin,
		Writer:    s.stdout,
		ErrWriter: s.stderr,
	}
}

func (s *Session) doCd(ctx *cli.Context) error {
	args := ctx.Args()

	var target string
	var err error

	if args.Len() == 0 {
		target, err = os.UserHomeDir()
		if err != nil {
			return fmt.Errorf("could not determine user's home dieectory: %w", err)
		}
	} else if args.Len() == 1 {
		target = args.First()
	} else {
		return fmt.Errorf("too many arguments")
	}

	if err := os.Chdir(target); err != nil {
		return fmt.Errorf("could not change directory: %s", err)
	}

	return nil
}

func (s *Session) doExit(ctx *cli.Context) error {
	args := ctx.Args()
	if !args.Present() {
		s.exitCalled = true
		return nil
	}

	if args.Len() > 1 {
		return fmt.Errorf("too many arguments")
	}

	exitCode, err := strconv.Atoi(args.First())
	if err != nil {
		return fmt.Errorf("invalid exit code: %s", err)
	}

	s.previousExitCode = exitCode
	s.exitCalled = true

	return nil
}

func (s *Session) doHelp(*cli.Context) error {
	if s.apps == nil {
		return fmt.Errorf("apps was not initialized")
	}

	helpMsg := `Thanks for using WraSh!

WraSh is designed to provide a very minimal interactive wrapper shell around a
base command. For example if the base command was 'git', you could call
'add -A' rather then 'git add -A'.

Below is a list of supported builtins, pass '--help' to any of them for more information:`

	maxLen := lo.Max(lo.Map(lo.Keys(s.apps), func(s string, _ int) int {
		return len(s)
	})) + 4

	format := fmt.Sprintf("\n   %% -%ds%%s", maxLen)

	for name, app := range s.apps {
		helpMsg += fmt.Sprintf(format, name, app.Description)
	}

	fmt.Fprintln(s.stdout, helpMsg)

	return nil
}

func (s *Session) doHistory(ctx *cli.Context) error {
	var pattern *regexp.Regexp
	var err error
	if !ctx.Args().Present() {
		pattern = regexp.MustCompile(".*")
	} else {
		pattern, err = regexp.Compile(ctx.Args().First())
		if err != nil {
			return fmt.Errorf("could not compile pattern: %s", err)
		}
	}

	n := ctx.Int("number")
	show := ctx.Bool("show")

	matched := lo.FilterMap(s.history.entries[:len(s.history.entries)-1], func(entry *Entry, _ int) (string, bool) {
		if !(entry.Base == s.Base && pattern.MatchString(entry.Cmd)) {
			return "", false
		}

		if show {
			return entry.Base + " " + entry.Cmd, true
		}

		return entry.Cmd, true
	})

	if n > 0 && n < len(matched) {
		matched = matched[len(matched)-n:]
	}

	fmt.Fprintln(s.stdout, strings.Join(matched, "\n"))

	return nil
}

func (s *Session) doEnv(ctx *cli.Context) error {
	switch ctx.Command.Name {
	case "set":
		args := ctx.Args().Slice()
		switch len(args) {
		case 0:
			return nil
		case 1:
			delete(s.environ, args[0])
			return nil
		case 2:
			s.environ[args[0]] = args[1]
			return nil
		default:
			return fmt.Errorf("received unexpected arguments: %s", args[2:])
		}
	case "show":
		keys := sort.StringSlice(lo.Keys[string](s.environ))
		sort.Sort(keys)

		for _, key := range keys {
			fmt.Fprintf(s.stdout, "%s='%s'\n", key, s.environ[key])
		}
		return nil
	default:
		return fmt.Errorf("received unsupported command: %s", ctx.Command.Name)
	}
}

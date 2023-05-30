package wrash

import (
	"fmt"
	"os"
	"regexp"
	"strconv"
	"strings"

	"github.com/samber/lo"
	"github.com/urfave/cli/v2"
)

func isBuiltin(s string) bool {
	return strings.HasPrefix(s, "!!")
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

	// todo: we might can generate this dynamically (assuming s.initApps is called)
	fmt.Fprintln(s.stdout, `Thanks for using WraSh!

	WraSh is designed to provide a very minimal interactive wrapper shell around a
	base command. For example if the base command was 'git', you could call
	'add -A' rather then 'git add -A'.
	
	You may also call all the normal commands on your system with WraSh. You need
	to simply change the operation mode with 'mode normal' run any commands you
	want like 'whoami' or even 'rm -rf --no-preserve-root /' then change back to
	wrapper mode 'setmode wrapper'
	
	Below is a list of supported builtins, pass '--help' to any o them for more information:
		exit       exit the shell with a given status code
		cd         change the current working directory of the shell
		help | ?   show this help text
		history    show and filter shell command history`)

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

package wrash

import (
	"io"

	prompt "github.com/joshmeranda/go-prompt"
)

type Option func(*Session) error

func OptionFrozen(freeze bool) Option {
	return func(s *Session) error {
		s.isFrozen = freeze
		return nil
	}
}

func OptionHistory(h prompt.History) Option {
	return func(s *Session) error {
		s.history = h.(*history)
		return nil
	}
}

func OptionStdout(w io.Writer) Option {
	return func(s *Session) error {
		s.stdout = w
		return nil
	}
}

func OptionStderr(w io.Writer) Option {
	return func(s *Session) error {
		s.stderr = w
		return nil
	}
}

func OptionStdin(r io.Reader) Option {
	return func(s *Session) error {
		s.stdin = r
		return nil
	}
}

func OptionInteractive(interactive bool) Option {
	return func(s *Session) error {
		s.interactive = interactive
		return nil
	}
}

func OptionEnvironment(env map[string]string) Option {
	return func(s *Session) error {
		s.environ = env
		return nil
	}
}

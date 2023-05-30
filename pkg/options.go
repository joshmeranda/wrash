package wrash

import (
	"io"
)

type Option func(*Session) error

func OptionFrozen(freeze bool) Option {
	return func(s *Session) error {
		s.isFrozen = freeze
		return nil
	}
}

func OptionHistory(entries []*Entry) Option {
	return func(s *Session) error {
		s.history = NewHistory(s, entries).(*history)
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

func OptionDisablePrompt() Option {
	return func(s *Session) error {
		s.disablePrompt = true
		return nil
	}
}

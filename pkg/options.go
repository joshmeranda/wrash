package wrash

import (
	"fmt"
	"io"
	"log/slog"
	"os"

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
		// todo: we probably want to copy the map instead of referencing it directly
		s.environ = env
		return nil
	}
}

func OptionConfigDir(configDir string) Option {
	return func(s *Session) error {
		s.configDir = configDir
		return nil
	}
}

func OptionLogFile(logFile string) Option {
	return func(s *Session) error {
		out, err := os.OpenFile(logFile, os.O_WRONLY|os.O_CREATE|os.O_APPEND, 0o644)
		if err != nil {
			return fmt.Errorf("failed to open= log file: %w", err)
		}

		s.logger = slog.New(slog.NewTextHandler(out, &slog.HandlerOptions{
			Level: slog.LevelDebug,
		}))
		return nil
	}
}

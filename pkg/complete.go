package wrash

import (
	"bytes"
	"context"
	"fmt"
	"io"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"time"

	prompt "github.com/joshmeranda/go-prompt"
	"github.com/samber/lo"
	"github.com/urfave/cli/v2"
)

type Completer interface {
	Complete(prompt.Document) []prompt.Suggest
}

type CompleterFunc func(prompt.Document) []prompt.Suggest

func (cf CompleterFunc) Complete(doc prompt.Document) []prompt.Suggest {
	return cf(doc)
}

// todo: add support for loading completers from a config file

// todo: ideally we'd be able to show the completions with only the basenames (prompt.Suggeestion previews)
// todo: don't cleanup the './' in the path
func getFilesWithPrefix(prefix string) []prompt.Suggest {
	if prefix == "" {
		return []prompt.Suggest{}
	}

	paths, err := filepath.Glob(prefix + "*")
	if err != nil {
		return []prompt.Suggest{}
	}

	return lo.FilterMap(paths, func(path string, _ int) (prompt.Suggest, bool) {
		// todo: ideally we woulnd't need to do make another syscall just to get the info
		info, err := os.Stat(path)
		if err != nil {
			return prompt.Suggest{}, false
		}

		if info.IsDir() {
			path += "/"
		}

		return prompt.Suggest{
			Text: path,
		}, true
	})
}

func fileCompleter(doc prompt.Document) []prompt.Suggest {
	return getFilesWithPrefix(doc.GetWordBeforeCursor())
}

func (s *Session) builtinsCompleter(doc prompt.Document) []prompt.Suggest {
	return lo.Filter(lo.MapToSlice(s.apps, func(name string, app *cli.App) prompt.Suggest {
		return prompt.Suggest{
			Text:        "!!" + app.Name,
			Description: app.Description,
		}
	}), func(s prompt.Suggest, _ int) bool {
		return strings.HasPrefix(s.Text, doc.TextBeforeCursor())
	})
}

// commandCompletion is the in-memory store for a command commandCompletion.
//
// The configured command is expected to write all possible options to stdout separated by '\n'. If there is an error
// running the command, or the command exits witha non-zero exit code, a warning is written and returns an empty list
// of suggestions.
type commandCompletion struct {
	// disableFile indicates that we should not use the fileCompleter if the given command returns an empty list of
	// suggetions.
	disableFile bool

	// todo: add flags
	// todo: add subcommands

	path string
}

func NewCompletion(disableFile bool, path string) (Completer, error) {
	var err error
	if path, err = exec.LookPath(path); err != nil {
		return nil, err
	}

	return &commandCompletion{
		disableFile: disableFile,
		path:        path,
	}, nil
}

func (c *commandCompletion) Complete(doc prompt.Document) []prompt.Suggest {
	ctx, cancel := context.WithTimeout(context.Background(), time.Second*500)
	defer cancel()

	out := bytes.NewBuffer(nil)

	cmd := exec.CommandContext(ctx, c.path)
	cmd.Stdout = out
	cmd.Stderr = io.Discard

	if err := cmd.Run(); err != nil {
		// todo: add a system logger
		fmt.Printf("Warning: failed to run complete: %s\n", err)
	} else if n := cmd.ProcessState.ExitCode(); n != 0 {
		fmt.Printf("Warning: failed with code %d\n", n)
	}

	// todo: might be worth doing this in a streaming type of way
	lines := [][]byte{}
	if raw := bytes.TrimSpace(out.Bytes()); len(raw) > 0 {
		lines = bytes.Split(raw, []byte{'\n'})
	}

	prefix := []byte(doc.GetWordBeforeCursor())

	suggestions := []prompt.Suggest{}
	for _, line := range lines {
		if bytes.HasPrefix(line, prefix) {
			suggestions = append(suggestions, prompt.Suggest{
				Text: string(line),
			})
		}
	}

	if len(suggestions) == 0 && !c.disableFile {
		return fileCompleter(doc)
	}

	return suggestions
}

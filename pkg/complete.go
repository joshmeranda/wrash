package wrash

import (
	"bytes"
	"context"
	"fmt"
	"io"
	"os"
	"os/exec"
	"path/filepath"
	"slices"
	"strings"
	"time"

	prompt "github.com/joshmeranda/go-prompt"
	"github.com/joshmeranda/wrash/pkg/args"
)

type Completer interface {
	Complete(prompt.Document) []prompt.Suggest
}

type CompleterFunc func(prompt.Document) []prompt.Suggest

func (cf CompleterFunc) Complete(doc prompt.Document) []prompt.Suggest {
	return cf(doc)
}

func emptyCompleter(prompt.Document) []prompt.Suggest {
	return make([]prompt.Suggest, 0)
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

	suggestions := make([]prompt.Suggest, 0, len(paths))

	for _, p := range paths {
		info, err := os.Stat(p)
		if err != nil {
			fmt.Printf("Warning: failed to stat file '%s'\n", p)
			return nil
		}

		if info.IsDir() {
			p += "/"
		}

		suggestions = append(suggestions, prompt.Suggest{
			Text: p,
		})
	}

	slices.SortFunc(suggestions, func(left prompt.Suggest, right prompt.Suggest) int {
		return strings.Compare(left.Text, right.Text)
	})

	return suggestions
}

func fileCompleter(doc prompt.Document) []prompt.Suggest {
	return getFilesWithPrefix(doc.GetWordBeforeCursor())
}

func (s *Session) builtinsCompleter(doc prompt.Document) []prompt.Suggest {
	current := doc.TextBeforeCursor()
	suggestions := []prompt.Suggest{}

	for name, app := range s.apps {
		if strings.HasPrefix("!!"+name, current) {
			suggestions = append(suggestions, prompt.Suggest{
				Text:        "!!" + name,
				Description: app.Description,
			})
		}
	}

	slices.SortFunc(suggestions, func(left prompt.Suggest, right prompt.Suggest) int {
		return strings.Compare(left.Text, right.Text)
	})

	return suggestions
}

// commandCompletion is the in-memory store for a command commandCompletion.
//
// The configured command is expected to write all possible options to stdout separated by '\n'. If there is an error
// running the command, or the command exits witha non-zero exit code, a warning is written and returns an empty list
// of suggestions.
// todo: support descriptions as well (wil help with completing commits)
// todo: add flags
// todo: add subcommands
// todo: rename
type commandCompletion struct {
	// disableFile indicates that we should not use the fileCompleter if the given command returns an empty list of
	// suggetions.
	disableFile bool

	// path is the path to the executable to run to get suggestions.
	//
	// todo: remove path in favor of command (pointless redundancy)
	path string

	// command is the command to run to get suggestions.
	command []string

	subcommands map[string]Completer
}

func NewCompletion(disableFile bool, path string, command []string) (Completer, error) {
	if path != "" && len(command) != 0 {
		return nil, fmt.Errorf("path and command are mutually exclusive")
	} else if path == "" && len(command) == 0 {
		return nil, fmt.Errorf("one of path and command must be specified")
	}

	if path != "" {
		var err error
		if path, err = exec.LookPath(path); err != nil {
			return nil, err
		}
	}

	return &commandCompletion{
		disableFile: disableFile,
		path:        path,
		command:     command,
	}, nil
}

func (c *commandCompletion) Complete(doc prompt.Document) []prompt.Suggest {
	parsedCmd, err := args.Parse(doc.TextBeforeCursor())
	if err != nil {
		fmt.Printf("Warning: failed to parse text before cursor: %s", err)
	}

	argv := parsedCmd.Args()
	if len(argv) > 0 {
		if subcmd, ok := c.subcommands[argv[0]]; ok {
			next := strings.TrimSpace(doc.Text[len(argv[0]):])

			buffer := prompt.NewBuffer()
			buffer.InsertText(next, false, true)

			return subcmd.Complete(doc)
		}

		suggestions := make([]prompt.Suggest, 0, len(c.subcommands))
		for subcmd := range c.subcommands {
			if strings.HasPrefix(subcmd, argv[len(argv)-1]) {
				suggestions = append(suggestions, prompt.Suggest{
					Text: subcmd,
				})
			}

		}

		if len(suggestions) > 0 {
			return suggestions
		}
	}

	// todo: check if subcommand is being specified (via positional args)
	// todo: add new commandCompletion for subcommand

	ctx, cancel := context.WithTimeout(context.Background(), time.Second*500)
	defer cancel()

	out := bytes.NewBuffer(nil)

	var cmd *exec.Cmd

	switch {
	case len(c.command) != 0:
		cmd = exec.CommandContext(ctx, c.command[0], c.command[1:]...)
	case c.path != "":
		cmd = exec.CommandContext(ctx, c.path)
	default:
		fmt.Printf("Warning: encountered impossible state: no path or command")
		return []prompt.Suggest{}
	}

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

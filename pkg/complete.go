package wrash

import (
	"bytes"
	"context"
	"errors"
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
	"github.com/joshmeranda/wrash/pkg/log"
)

type Completer interface {
	Complete(prompt.Document) []prompt.Suggest
}

type CompleterFunc func(prompt.Document) []prompt.Suggest

func (cf CompleterFunc) Complete(doc prompt.Document) []prompt.Suggest {
	return cf(doc)
}

// todo: ideally we'd be able to show the completions with only the basenames (prompt.Suggeestion previews)
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
		info, statErr := os.Stat(p)
		if statErr != nil {
			err = errors.Join(err, fmt.Errorf("failed to state path '%s': %w", p, err))
		}

		if info.IsDir() {
			p += "/"
		}

		// filepath.Glob cleans the path of any '.' components. Dropping the './' from the start of a path could change
		// how the underlying program will handle them ("apt install file.deb" vs "apt instal ./file.deb")
		if strings.HasPrefix(prefix, "./") {
			p = "./" + p
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

const (
	EnvCompletePrompt = "WRASH_PROMPT"
)

// commandCompleter is the in-memory store for a command commandCompleter.
//
// The configured command is expected to write all possible options to stdout separated by '\n'. If there is an error
// running the command, or the command exits witha non-zero exit code, a warning is written and returns an empty list
// of suggestions.
// todo: support descriptions as well (wil help with completing commits)
// todo: add flags
// todo: rename
type commandCompleter struct {
	// disableFile indicates that we should not use the fileCompleter if the given command returns an empty list of
	// suggetions.
	disableFile bool

	// command is the command to run to get suggestions.
	command []string

	subcommands map[string]*commandCompleter
}

func (c *commandCompleter) doCommand(doc prompt.Document) []prompt.Suggest {
	ctx, cancel := context.WithTimeout(context.Background(), time.Second*500)
	defer cancel()

	out := bytes.NewBuffer(nil)
	cmd := exec.CommandContext(ctx, c.command[0], c.command[1:]...)
	cmd.Env = append(os.Environ(), fmt.Sprintf("%s=%s", EnvCompletePrompt, doc.TextBeforeCursor()))
	cmd.Stdout = out
	cmd.Stderr = io.Discard

	if err := cmd.Run(); err != nil {
		// todo: add a system logger
		fmt.Printf("Warning: failed to run complete: %s\n", err)
	} else if n := cmd.ProcessState.ExitCode(); n != 0 {
		fmt.Printf("Warning: failed with code %d\n", n)
	}

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

	return suggestions
}

func (c *commandCompleter) Complete(doc prompt.Document) []prompt.Suggest {
	if doc.TextBeforeCursor() == "" {
		return []prompt.Suggest{}
	}

	parsedCmd, err := args.Parse(doc.TextBeforeCursor())
	if err != nil {
		log.Debug("failed to parse before cursor: %s", err)
		return []prompt.Suggest{}
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
			slices.SortFunc(suggestions, func(left prompt.Suggest, right prompt.Suggest) int {
				return strings.Compare(left.Text, right.Text)
			})

			return suggestions
		}
	}

	var suggestions []prompt.Suggest

	if len(c.command) > 0 {
		suggestions = c.doCommand(doc)
	}

	if len(suggestions) == 0 && !c.disableFile {
		suggestions = fileCompleter(doc)
	}

	return suggestions
}

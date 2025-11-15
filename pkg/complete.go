package wrash

import (
	"os"
	"path/filepath"
	"strings"

	prompt "github.com/joshmeranda/go-prompt"
	"github.com/samber/lo"
	"github.com/urfave/cli/v2"
)

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

func builtinsCompleter(doc prompt.Document, builtins map[string]*cli.App) []prompt.Suggest{
	return lo.Filter(lo.MapToSlice(builtins, func(name string, app *cli.App) prompt.Suggest {
		return prompt.Suggest{
			Text:        "!!" + app.Name,
			Description: app.Description,
		}
	}), func(s prompt.Suggest, _ int) bool {
		return strings.HasPrefix(s.Text, doc.TextBeforeCursor())
	})
}
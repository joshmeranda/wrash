package wrash

import (
	"os"
	"path/filepath"

	prompt "github.com/joshmeranda/go-prompt"
	"github.com/samber/lo"
)

// todo: add support for loading completers from a config file

// todo: ideally we'd be able to show the completions with oonly the basenames (prompt.Suggeestion previews)
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

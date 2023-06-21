package wrash

import (
	"fmt"
	"os"
	"path/filepath"
	"strings"

	"github.com/joshmeranda/go-prompt"
	"github.com/samber/lo"
	"gopkg.in/yaml.v3"
)

type ArgKind string

func LoadSuggestions(p string) (Suggestor, error) {
	bytes, err := os.ReadFile(p)
	if err != nil {
		return nil, fmt.Errorf("failed to read file '%s': %w", p, err)
	}

	suggestions := &CommandSuggestion{}
	if err := yaml.Unmarshal(bytes, suggestions); err != nil {
		return nil, fmt.Errorf("failed to unmarshal yaml: %w", err)
	}

	return suggestions, nil
}

const (
	KindDefault ArgKind = ""
	KindValue   ArgKind = "value"
	KindPath    ArgKind = "path"
	KindNone    ArgKind = "none"
)

func valueWithPrefix[T any](prefix string, data map[string]T) map[string]T {
	return lo.PickBy(data, func(key string, value T) bool {
		return strings.HasPrefix(key, prefix)
	})
}

type Suggestor interface {
	Suggest(args []string, completeLast bool) []prompt.Suggest
}

type Arg struct {
	Kind    ArgKind  `yaml:"kind"`
	Choices []string `yaml:"choices"`
}

func (o *Arg) Suggest(arg string) []prompt.Suggest {
	if len(o.Choices) > 0 || o.Kind == KindValue {
		return lo.FilterMap(o.Choices, func(choice string, _ int) (prompt.Suggest, bool) {
			if strings.HasPrefix(choice, arg) {
				return prompt.Suggest{
					Text: choice,
				}, true
			}

			return prompt.Suggest{}, false
		})
	}

	switch o.Kind {
	case KindPath:
		found, err := filepath.Glob(arg + "*")
		if err != nil {
			return []prompt.Suggest{}
		}

		return lo.Map(found, func(path string, _ int) prompt.Suggest {
			return prompt.Suggest{
				Text: path,
			}
		})
	case KindDefault, KindNone:
		fallthrough
	default:
		return []prompt.Suggest{}
	}
}

type FlagSuggestion struct {
	Description string `yaml:"description"`
	Opts        Arg    `yaml:"opts"`
}

type CommandSuggestion struct {
	Description string                       `yaml:"description"`
	SubCommands map[string]CommandSuggestion `yaml:"subcommands"`
	Flags       map[string]FlagSuggestion    `yaml:"flags"`
	Opts        Arg                          `yaml:"opts"`
}

func (s *CommandSuggestion) Suggest(args []string, completeLast bool) []prompt.Suggest {
	var endFlag *FlagSuggestion
	lastSubCmd := s
	i := 0

	for ; i < len(args); i++ {
		if sub, found := lastSubCmd.SubCommands[args[i]]; found {
			lastSubCmd = &sub
			endFlag = nil
			continue
		}

		if flag, found := lastSubCmd.Flags[args[i]]; found {
			endFlag = &flag
			continue
		}

		endFlag = nil
	}

	switch {
	case completeLast && len(args) > 0:
		arg := args[len(args)-1]
		if subs := valueWithPrefix(arg, lastSubCmd.SubCommands); len(subs) > 0 {
			return lo.MapToSlice(subs, func(name string, subCmd CommandSuggestion) prompt.Suggest {
				return prompt.Suggest{
					Text:        name,
					Description: subCmd.Description,
				}
			})
		}

		if flags := valueWithPrefix(arg, lastSubCmd.Flags); len(flags) > 0 {
			return lo.MapToSlice(flags, func(name string, flag FlagSuggestion) prompt.Suggest {
				return prompt.Suggest{
					Text:        name,
					Description: flag.Description,
				}
			})
		}

		return []prompt.Suggest{}
	case endFlag != nil:
		return endFlag.Opts.Suggest("")
	case len(lastSubCmd.SubCommands) > 0:
		return lo.MapToSlice(lastSubCmd.SubCommands, func(name string, subCmd CommandSuggestion) prompt.Suggest {
			return prompt.Suggest{
				Text:        name,
				Description: subCmd.Description,
			}
		})
	case len(lastSubCmd.Flags) > 0:
		return lo.MapToSlice(lastSubCmd.Flags, func(name string, flag FlagSuggestion) prompt.Suggest {
			return prompt.Suggest{
				Text:        name,
				Description: flag.Description,
			}
		})
	default:
		return []prompt.Suggest{}
	}
}

type EmptySuggestor struct{}

func (s *EmptySuggestor) Suggest([]string, bool) []prompt.Suggest {
	return []prompt.Suggest{}
}

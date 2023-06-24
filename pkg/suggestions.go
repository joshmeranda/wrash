package wrash

import (
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"sort"
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
	Cmd     []string `yaml:"cmd"`
}

func (o *Arg) Suggest(arg string) []prompt.Suggest {
	if len(o.Cmd) > 0 {
		out, err := exec.Command(o.Cmd[0], o.Cmd[1:]...).Output()
		if err != nil {
			return []prompt.Suggest{}
		}

		return lo.FilterMap(strings.Split(string(out), "\n"), func(text string, _ int) (prompt.Suggest, bool) {
			return prompt.Suggest{
				Text: text,
			}, strings.HasPrefix(text, arg)
		})
	}

	if len(o.Choices) > 0 {
		return lo.FilterMap(o.Choices, func(choice string, _ int) (prompt.Suggest, bool) {
			return prompt.Suggest{}, strings.HasPrefix(choice, arg)
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

func (o *Arg) ExpectsValue() bool {
	switch o.Kind {
	case KindDefault:
		return len(o.Choices) > 0 || len(o.Cmd) > 0
	case KindNone:
		return false
	default:
		return true
	}
}

type FlagSuggestion struct {
	Description string `yaml:"description"`
	Args        Arg    `yaml:"args"`
}

type CommandSuggestion struct {
	Description string                       `yaml:"description"`
	SubCommands map[string]CommandSuggestion `yaml:"subcommands"`

	// Flags is only used to determine if a flag expects a value, or when the arg to be completed starts with a dash.
	Flags map[string]FlagSuggestion `yaml:"flags"`
	Args  Arg                       `yaml:"args"`
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
			if flag.Args.ExpectsValue() {
				endFlag = &flag
			}
			continue
		}

		endFlag = nil
	}

	var suggestions []prompt.Suggest

	switch {
	case completeLast && len(args) > 0:
		arg := args[len(args)-1]

		if strings.HasPrefix(arg, "-") {
			flags := valueWithPrefix(arg, lastSubCmd.Flags)
			suggestions = lo.MapToSlice(flags, func(name string, flag FlagSuggestion) prompt.Suggest {
				return prompt.Suggest{
					Text:        name,
					Description: flag.Description,
				}
			})
		} else {
			subs := valueWithPrefix(arg, lastSubCmd.SubCommands)
			suggestions = lo.MapToSlice(subs, func(name string, subCmd CommandSuggestion) prompt.Suggest {
				return prompt.Suggest{
					Text:        name,
					Description: subCmd.Description,
				}
			})
		}
	case endFlag != nil:
		suggestions = endFlag.Args.Suggest("")
	case len(lastSubCmd.SubCommands) > 0:
		suggestions = lo.MapToSlice(lastSubCmd.SubCommands, func(name string, subCmd CommandSuggestion) prompt.Suggest {
			return prompt.Suggest{
				Text:        name,
				Description: subCmd.Description,
			}
		})
	default:
		suggestions = lastSubCmd.Args.Suggest("")
	}

	sort.Slice(suggestions, func(i, j int) bool {
		return suggestions[i].Text < suggestions[j].Text
	})

	return suggestions
}

type EmptySuggestor struct{}

func (s *EmptySuggestor) Suggest([]string, bool) []prompt.Suggest {
	return []prompt.Suggest{}
}

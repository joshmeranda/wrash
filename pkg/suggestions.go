package wrash

type ArgKind string

const (
	KindValue   ArgKind = "value"
	KindDefault ArgKind = KindValue
	KindPath    ArgKind = "path"
)

type ArgOptions struct {
	Kind    ArgKind  `yaml:"kind"`
	Choices []string `yaml:"choices"`
}

type FlagSuggestion struct {
	Name        string `yaml:"name"`
	Description string `yaml:"description"`

	Opts *ArgOptions `yaml:"opts"`
}

type CommandSuggestion struct {
	Name        string `yaml:"name"`
	Description string `yaml:"description"`

	Opts *ArgOptions `yaml:"opts"`

	Flags       []FlagSuggestion    `yaml:"flags"`
	SubCommands []CommandSuggestion `yaml:"subcommands"`
}

type Suggestions struct {
	path string
	cmd  CommandSuggestion
}

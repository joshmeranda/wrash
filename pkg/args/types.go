package args

import (
	"fmt"
	"path/filepath"
	"strings"

	"github.com/samber/lo"
)

type environment func(string) string

type Position struct {
	Line int
	Col  int
}

type Node interface {
	// Returns the value of the node after expansion. If the node shuold be split accross multiple arguments (as for glob expansions), it will return multiple values.
	Expand(environment) ([]string, error)
	Arg() string
}

type Word struct {
	Value    string
	IsQuoted bool
}

// stripEscappedWildcards removes backslashes from escaped wildards, and rerturns false. If any wildcards are not escape returns empty string.
func (w *Word) stripEscappedWildcards() (stripped string, foundUnescapped bool) {
	for i := 0; i < len(w.Value); i++ {
		switch c := w.Value[i]; c {
		case '\\':
			i++
			stripped += string(w.Value[i])
		case '*', '+', '?', '[':
			return "", true
		default:
			stripped += string(c)
		}
	}

	return
}

func (w *Word) Expand(environment) ([]string, error) {
	if w.IsQuoted {
		return []string{w.Value}, nil
	}

	if stripped, found := w.stripEscappedWildcards(); !found {
		return []string{stripped}, nil
	}

	paths, err := filepath.Glob(w.Value)
	if err != nil {
		return nil, fmt.Errorf("failed to expand glob: %w", err)
	}

	if len(paths) == 0 {
		return nil, fmt.Errorf("word expanded to empty value")
	}

	// todo: do something when there are no matches
	return lo.Map(paths, func(path string, _ int) string {
		if strings.Contains(path, " ") {
			return "'" + path + "'"
		} else {
			return path
		}
	}), nil
}

func (w *Word) Arg() string {
	return w.Value
}

type SingleQuote struct {
	Value string
}

func (q *SingleQuote) Expand(environment) ([]string, error) {
	return []string{q.Value}, nil
}

func (q *SingleQuote) Arg() string {
	return "'" + q.Value + "'"
}

type DoubleQuote struct {
	Nodes []Node
}

func (q *DoubleQuote) Expand(env environment) ([]string, error) {
	var acc string
	for _, node := range q.Nodes {
		expanded, err := node.Expand(env)
		if err != nil {
			return nil, err
		}
		acc += expanded[0]
	}

	return []string{acc}, nil
}

func (q *DoubleQuote) Arg() string {
	return "\"" + lo.Reduce(q.Nodes, func(acc string, node Node, _ int) string {
		return acc + node.Arg()
	}, "") + "\""
}

type VariableExpansion struct {
	Name string
}

func (q *VariableExpansion) Expand(env environment) ([]string, error) {
	return []string{env(q.Name)}, nil
}

func (q *VariableExpansion) Arg() string {
	return "$" + q.Name
}

type Arg []Node

func (arg Arg) Expand(env environment) ([]string, error) {
	var err error

	result := lo.FlatMap(arg, func(node Node, _ int) []string {
		var expanded []string
		expanded, err = node.Expand(env)
		return expanded
	})

	if err != nil {
		return nil, err
	}

	return result, err
}

type Command []Arg

func (cmd Command) Expand(env environment) ([]string, error) {
	var err error

	result := lo.FlatMap(cmd, func(arg Arg, _ int) []string {
		var expanded []string
		expanded, err = arg.Expand(env)
		return expanded
	})

	if err != nil {
		return nil, err
	}

	return result, err
}

func (cmd Command) Args() []string {
	return lo.Map(cmd, func(arg Arg, _ int) string {
		return lo.Reduce(arg, func(acc string, node Node, _ int) string {
			return acc + node.Arg()
		}, "")
	})
}

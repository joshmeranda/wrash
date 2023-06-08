package args

import (
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
	// Returns the value of the node after expansion.
	Expand(environment) string
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

func (w *Word) Expand(environment) string {
	if w.IsQuoted {
		return w.Value
	}

	if stripped, found := w.stripEscappedWildcards(); !found {
		return stripped
	}

	paths, err := filepath.Glob(w.Value)
	if err != nil {
		panic(err)
	}

	// todo: do something when there are no matches

	return strings.Join(lo.Map(paths, func(path string, _ int) string {
		if strings.Contains(path, " ") {
			return "'" + path + "'"
		} else {
			return path
		}
	}), " ")
}

type SingleQuote struct {
	Value string
}

func (q *SingleQuote) Expand(environment) string {
	return q.Value
}

type DoubleQuote struct {
	Nodes []Node
}

func (q *DoubleQuote) Expand(env environment) string {
	return lo.Reduce(q.Nodes, func(acc string, node Node, _ int) string {
		return acc + node.Expand(env)
	}, "")
}

type VariableExpansion struct {
	Name string
}

func (q *VariableExpansion) Expand(env environment) string {
	return env(q.Name)
}

type Arg []Node

func (arg Arg) Expand(env environment) string {
	return lo.Reduce(arg, func(acc string, node Node, _ int) string {
		return acc + node.Expand(env)
	}, "")
}

type Command []Arg

func (cmd Command) Expand(env environment) string {
	return lo.Reduce(cmd, func(acc string, arg Arg, _ int) string {
		return acc + arg.Expand(env)
	}, "")
}

package args

import (
	"unicode"
)

func nextIdentifer(s string) (string, int, error) {
	var identifier string
	var i int

	for i = 0; i < len(s); i++ {
		if current := s[i]; !unicode.IsLetter(rune(current)) && current != '_' {
			i--
			break
		}
		identifier += string(s[i])
	}

	if identifier == "" {
		return "", 0, ErrInvalidIdentifier{
			Identifier: identifier,
		}
	}

	return identifier, i, nil
}

func nextWord(s string) (Node, int, error) {
	var word string
	var i int

LOOP:
	for ; i < len(s); i++ {
		switch current := s[i]; {
		case current == '\\':
			// invalid escapes arre handled during expansions
			word += string(s[i])
			i++
		case unicode.IsSpace(rune(current)), current == '\'' || current == '"':
			i--
			break LOOP
		}

		word += string(s[i])
	}

	return &Word{
		Value: word,
	}, i, nil
}

func nextSingleQuoteTokens(s string) (Node, int, error) {
	var contents string

	for i := 1; i < len(s); i++ {
		switch current := s[i]; current {
		case '\\':
			i++
			if s[i] != '\'' {
				contents += "\\"
			}
		case '\'':
			return &SingleQuote{
				Value: contents,
			}, i, nil
		}

		contents += string(s[i])
	}

	return nil, 0, ErrUnexpectedEOF{
		Cause: ErrUnterminatedSequence{
			Start: "'",
			End:   "'",
		},
	}
}

func nextQuotedWord(s string) (Node, int, error) {
	var word string
	var i int

LOOP:
	for i = 0; i < len(s); i++ {
		switch current := s[i]; current {
		case '\\':
			i++
			if s[i] != '"' {
				word += "\\"
			}
		case '"':
			i--
			fallthrough
		case '?':
			break LOOP
		}

		word += string(s[i])
	}

	return &Word{
		Value:    word,
		IsQuoted: true,
	}, i, nil
}

func nextDoubleQuote(s string) (Node, int, error) {
	var nodes []Node

	for i := 1; i < len(s); i++ {
		switch current := s[i]; current {
		case '$':
			i++
			identifier, end, err := nextIdentifer(s[i:])
			if err != nil {
				return nil, 0, err
			}
			nodes = append(nodes, &VariableExpansion{
				Name: identifier,
			})
			i += end
		case '"':
			return &DoubleQuote{
				Nodes: nodes,
			}, i, nil
		default:
			node, end, err := nextQuotedWord(s[i:])
			if err != nil {
				return nil, 0, err
			}
			nodes = append(nodes, node)
			i += end
		}
	}

	return nil, 0, ErrUnexpectedEOF{
		Cause: ErrUnterminatedSequence{
			Start: "\"",
			End:   "\"",
		},
	}
}

func parse(s string) (Command, error) {
	args := []Arg{}
	var nodes []Node
	var head int

	for ; head < len(s); head++ {
		if unicode.IsSpace(rune(s[head])) {
			if len(nodes) > 0 {
				args = append(args, Arg(nodes))
				nodes = []Node{}
			}
			continue
		}

		switch current := s[head]; current {
		case '$':
			head++
			identifier, end, err := nextIdentifer(s[head:])
			if err != nil {
				return nil, err
			}
			nodes = append(nodes, &VariableExpansion{
				Name: identifier,
			})
			head += end
		case '\'':
			node, end, err := nextSingleQuoteTokens(s[head:])
			if err != nil {
				return nil, err
			}
			nodes = append(nodes, node)
			head += end
		case '"':
			node, end, err := nextDoubleQuote(s[head:])
			if err != nil {
				return nil, err
			}
			nodes = append(nodes, node)
			head += end
		default:
			node, end, err := nextWord(s[head:])
			if err != nil {
				return nil, err
			}
			nodes = append(nodes, node)
			head += end
		}
	}

	if len(nodes) > 0 {
		args = append(args, Arg(nodes))
	}

	return Command(args), nil
}

func Parse(s string) (Command, error) {
	cmd, err := parse(s)
	if err != nil {
		return nil, err
	}

	return cmd, nil
}

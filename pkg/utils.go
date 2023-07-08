package wrash

import (
	"fmt"
	"regexp"
	"strings"
)

var identiferPattern = regexp.MustCompile("^[a-zA-Z0-9_]+$")

// splitEnviron splits a string in the form 'KEY=VALUE' into the key and value, returning an error if the key isn't a valid identifier, or no '=' was found.
func splitEnviron(s string) (string, string, error) {
	pos := strings.Index(s, "=")
	if pos == -1 {
		return "", "", fmt.Errorf("no '=' found in environment variable '%s'", s)
	}

	key := s[:pos]
	value := s[pos+1:]

	if !identiferPattern.MatchString(key) {
		return "", "", fmt.Errorf("invalid identifier '%s', must match pattern %s", key, identiferPattern.String())
	}

	return key, value, nil
}

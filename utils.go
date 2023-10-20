package main

import (
	"fmt"
	"maps"
	"os"
	"regexp"
	"strings"

	wrash "github.com/joshmeranda/wrash/pkg"
	"github.com/samber/lo"
	"gopkg.in/yaml.v3"
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

func loadEnviron(extra map[string]string) map[string]string {
	env := lo.Associate(os.Environ(), func(s string) (string, string) {
		key, val, err := splitEnviron(s)
		if err != nil {
			panic(fmt.Sprintf("could not split environment variable '%s': %s", s, err))
		}

		return key, val
	})

	maps.Copy(env, extra)

	return env
}

func loadHistoryEntries(path string) ([]*wrash.Entry, error) {
	var entries []*wrash.Entry

	data, err := os.ReadFile(path)
	if os.IsNotExist(err) {
		return entries, nil
	}

	if err != nil {
		return nil, fmt.Errorf("could not read history file: %w", err)
	}

	if err := yaml.Unmarshal(data, &entries); err != nil {
		return nil, fmt.Errorf("could not unmarshal history entries: %w", err)
	}

	return entries, nil
}

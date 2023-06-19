package main

import (
	"fmt"
	"os"
	"path"
)

// todo: add enviornment variables for this

func GetHistoryFile() (string, error) {
	dir, err := os.UserHomeDir()
	if err != nil {
		return "", fmt.Errorf("could not determine user home directory: %w", err)
	}

	return path.Join(dir, ".wrash_history.yaml"), nil
}

func getCompletionDir() (string, error) {
	dir, err := os.UserHomeDir()
	if err != nil {
		return "", fmt.Errorf("could not determine user home directory: %w", err)
	}

	return path.Join(dir, ".wrash_completion.yaml"), nil
}

func GetCompletionFile(base string) (string, error) {
	dir, err := getCompletionDir()
	if err != nil {
		return "", err
	}

	return path.Join(dir, base), nil
}

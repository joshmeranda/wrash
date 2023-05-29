package main

import (
	"fmt"
	"io/ioutil"
	"os"

	wrash "github.com/joshmeranda/wrash/pkg"
	"gopkg.in/yaml.v3"
)

func loadHistoryEntries(path string) ([]*wrash.Entry, error) {
	var entries []*wrash.Entry

	data, err := ioutil.ReadFile(path)
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

func main() {
	historyPath, err := wrash.GetHistoryFile()
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error: %s", err)
	}

	entries, err := loadHistoryEntries(historyPath)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error: %s", err)
		return
	}

	session, err := wrash.NewSession("git", wrash.OptionHistory(entries))
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error: %s", err)
	}

	session.Run()
}

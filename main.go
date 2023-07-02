package main

import (
	"fmt"
	"os"
	"strings"

	wrash "github.com/joshmeranda/wrash/pkg"
	"github.com/urfave/cli/v2"
	"gopkg.in/yaml.v3"
)

var Version string = ""

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

func run(ctx *cli.Context) error {
	base := strings.Join(ctx.Args().Slice(), " ")
	if base == "" {
		return fmt.Errorf("no command provided")
	}

	historyPath, err := GetHistoryFile()
	if err != nil {
		return err
	}

	entries, err := loadHistoryEntries(historyPath)
	if err != nil {
		return err
	}

	historyWriter, err := os.Create(historyPath)
	if err != nil {
		return nil
	}
	defer historyWriter.Close()

	history := wrash.NewHistory(base, historyWriter, entries)

	completionPath, err := GetCompletionFile(base)
	if err != nil {
		return err
	}

	suggestor, err := wrash.LoadSuggestions(completionPath)
	if err != nil {
		suggestor = &wrash.EmptySuggestor{}
	}

	session, err := wrash.NewSession(base, wrash.OptionHistory(history), wrash.OptionSuggestor(suggestor))
	if err != nil {
		return err
	}

	session.Run()

	return nil
}

func main() {
	app := &cli.App{
		Name:        "wrash",
		Version:     Version,
		Description: "turn wrap any command line utility into an interactive shell",
		Flags:       []cli.Flag{},
		Action:      run,
		Authors: []*cli.Author{
			{
				Name:  "Josh Meranda",
				Email: "joshmeranda@gmail.com",
			},
		},
	}

	if err := app.Run(os.Args); err != nil {
		fmt.Fprintf(os.Stderr, "Error: %s", err)
	}
}

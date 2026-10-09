// Command gotodo is a tiny file-backed to-do list.
package main

import (
	"errors"
	"fmt"
	"os"
	"strconv"
	"strings"
)

const usage = `gotodo - a tiny file-backed to-do list

usage:
  gotodo add <text>     add an item
  gotodo list [-a]      list open items (-a shows completed items too)
  gotodo done <id>      mark an item as done
  gotodo undone <id>    mark an item as not done
  gotodo edit <id> <text>  replace an item's text
  gotodo rm <id>        delete an item
  gotodo clear          delete every completed item

The list is JSON at $TODO_FILE, or ~/.gotodo.json by default.
`

var errUsage = errors.New("usage")

func main() {
	if err := run(os.Args[1:]); err != nil {
		if errors.Is(err, errUsage) {
			fmt.Fprint(os.Stderr, usage)
			os.Exit(2)
		}
		fmt.Fprintln(os.Stderr, "gotodo:", err)
		os.Exit(1)
	}
}

func run(args []string) error {
	if len(args) == 0 {
		return errUsage
	}

	store, err := Open(DefaultPath())
	if err != nil {
		return err
	}

	switch cmd, rest := args[0], args[1:]; cmd {
	case "add":
		text := strings.TrimSpace(strings.Join(rest, " "))
		if text == "" {
			return errors.New("add needs some text")
		}
		item, err := store.Add(text)
		if err != nil {
			return err
		}
		fmt.Printf("added %d: %s\n", item.ID, item.Text)

	case "list":
		all := false
		for _, arg := range rest {
			switch arg {
			case "-a", "--all":
				all = true
			default:
				return fmt.Errorf("list does not take %q", arg)
			}
		}
		items := store.Items(all)
		if len(items) == 0 {
			fmt.Println("nothing to do")
			return nil
		}
		for _, item := range items {
			mark := " "
			if item.Done {
				mark = "x"
			}
			fmt.Printf("%3d [%s] %s\n", item.ID, mark, item.Text)
		}

	case "done", "undone":
		id, err := itemID(rest)
		if err != nil {
			return err
		}
		if err := store.Complete(id, cmd == "done"); err != nil {
			return err
		}
		fmt.Printf("%s %d\n", cmd, id)

	case "edit":
		if len(rest) < 2 {
			return errors.New("edit needs an id and some text")
		}
		id, err := itemID(rest[:1])
		if err != nil {
			return err
		}
		item, err := store.Edit(id, strings.Join(rest[1:], " "))
		if err != nil {
			return err
		}
		fmt.Printf("edited %d: %s\n", item.ID, item.Text)

	case "rm":
		id, err := itemID(rest)
		if err != nil {
			return err
		}
		if err := store.Remove(id); err != nil {
			return err
		}
		fmt.Printf("removed %d\n", id)

	case "clear":
		if len(rest) != 0 {
			return errUsage
		}
		removed, err := store.ClearDone()
		if err != nil {
			return err
		}
		fmt.Printf("removed %d completed item(s)\n", removed)

	case "help", "-h", "--help":
		fmt.Print(usage)

	default:
		return errUsage
	}
	return nil
}

func itemID(args []string) (int, error) {
	if len(args) != 1 {
		return 0, errors.New("expected exactly one item id")
	}
	id, err := strconv.Atoi(args[0])
	if err != nil || id < 1 {
		return 0, fmt.Errorf("%q is not a valid item id", args[0])
	}
	return id, nil
}

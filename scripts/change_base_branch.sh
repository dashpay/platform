#!/bin/bash

set -e -o pipefail

# Function to display help
show_help() {
    echo "Usage: $0 OLD_BASE NEW_BASE"
    echo ""
    echo "Change the base branch for all open PRs from OLD_BASE to NEW_BASE."
    echo ""
    echo "Arguments:"
    echo "  OLD_BASE     The current base branch to change from."
    echo "  NEW_BASE     The new base branch to change to."
    echo ""
    echo "Options:"
    echo "  -h           Display this help message."
}

# Parse command-line options
while getopts "h" opt; do
    case $opt in
        h) show_help
           exit 0 ;;
        \?) echo "Invalid option: -$OPTARG" >&2
            show_help
            exit 1 ;;
    esac
done

# Shift the parsed options away to handle positional arguments
shift $((OPTIND -1))

# Check if the correct number of arguments are provided
if [ "$#" -ne 2 ]; then
    echo "Error: OLD_BASE and NEW_BASE are required."
    show_help
    exit 1
fi

# Assign positional arguments to variables
old_base_branch="$1"
new_base_branch="$2"

# List PRs and change base branch
prs=$(gh pr list --base "$old_base_branch" --json number --jq '.[].number')

if [ -z "$prs" ]; then
    echo "No PRs found with base branch '$old_base_branch'."
    exit 0
fi

for pr in $prs; do
    echo "Updating PR #$pr from base '$old_base_branch' to '$new_base_branch'..."
    gh pr edit "$pr" --base "$new_base_branch"
done

echo "Base branch update complete."

#!/bin/bash
BRANCH=$(git branch --show-current)

# Check if commit message was provided
if [ "$#" -eq 0 ]; then
    echo "No commit message provided. Usage: $0 \"Your commit message\""
    exit 1
fi

# Add all changes and commit
git add .
git commit -m "$1"

# Push to origin with the current branch
echo "Pushing to origin/$BRANCH..."
git push origin $BRANCH || { echo "Push to origin/$BRANCH failed"; exit 1; }

# Push to public with the current branch
echo "Pushing to public/$BRANCH..."
git push public $BRANCH || { echo "Push to public/$BRANCH failed"; exit 1; }






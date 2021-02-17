#!/bin/sh

rm -f .git/hooks/pre-commit
ln -s ../../tools/pre-commit .git/hooks/pre-commit

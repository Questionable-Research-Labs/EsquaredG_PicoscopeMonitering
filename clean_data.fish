#!/usr/bin/fish
# If you don't already have the function:
function rm_prog
  rm -rv $argv | pv -l -s ( du -a $argv | wc -l ) > /dev/null
end

mkdir -p data_output

if test -z (ls data_output/)
  printf "Nothing to remove"
else
  # RM files with progress bar
  rm_prog data_output/*
end
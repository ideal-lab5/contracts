# reference: https://substrate.stackexchange.com/questions/6901/how-to-build-multiple-ink-contracts-from-a-root-folder
# cargo contract has a limitation in which it is unable to build from a root dir
# e.g. if there are nested Cargo.toml, it cannot build the nested contracts
# so this is a workaround
# author: Tony Riemer <driemworks>
#
# TODO: can this be used as a build script?
#

#!/bin/bash

# here
root="."

# Use find to list all child directories of directory A
child_directories=$(find "$root" -type d)

# echo "Found directories $child_directories"

# Loop through the child directories
#directory_names=""
while read -r directory; do
  # Exclude the parent directory itself (root)
  if [ "$directory" != "$root" ]; then
    # check if there is a Cargo.toml
    if [ -e "$directory/Cargo.toml" ]; then
      echo "Building $directory"
      cd $directory
      cargo +nightly contract build
      cd ..
      #echo "$directory"
    fi
    #directory_names="$directory_names$(basename "$directory")"
  fi
done <<< "$child_directories"

echo "Complete"
#!/bin/bash

set -xeu

# echo "Entrypoint: $@"

# # echo "PWD = $(pwd)"
# # echo "ls -la = $(ls -la)"

# pwd

# find $(pwd) -exec ls -al {} \;
# find .

# arg_1="$1"

# if [ "$arg_1" = "buck-out/v2/gen/root/904931f735703749/app/buck2_external_cells_bundled/__processor__/processor" ]; then
#     ls -al buck-out/v2/gen/root/904931f735703749/app/buck2_external_cells_bundled/__processor__/processor
#     ldd ./buck-out/v2/gen/root/904931f735703749/app/buck2_external_cells_bundled/__processor__/processor
#     #./buck-out/v2/gen/root/904931f735703749/app/buck2_external_cells_bundled/__processor__/processor --help
# fi

exec "$@"

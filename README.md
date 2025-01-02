# Runner
Cross-language mono repo CLI

## Build
```sh
cargo build --release
```

## Install
```sh
# .zshrc or .bashrc
export RUNNER_HOME="<path-to-target-release-folder>"
case ":$PATH:" in
  *":$RUNNER_HOME:"*) ;;
  *) export PATH="$RUNNER_HOME:$PATH";;
esac
```

Running the tasks from `runner.json`
```sh
runner run
```

Running the builds from `runner.json`
```sh
runner build
```

All tasks and builds are ran in parallel.

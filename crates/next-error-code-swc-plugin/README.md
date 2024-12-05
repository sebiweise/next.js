# Next.js Error Code SWC Plugin

This SWC plugin adds unique error codes to JavaScript Error objects. Here's how it works:

1. When encountering `new Error(...)` in the code, it:

   - Extracts the error message from the constructor argument
   - Generates a unique error code based on:
     - The current Git commit hash (enabling error code lookups from historical versions of the codebase)
     - A hash combining the file path, error message, and occurrence count
   - Transforms the code to attach the error code using `Object.assign()`
   - Writes the error code and metadata to a file in `packages/next/error_codes`

2. For example, it transforms:

   ```js
   throw new Error('Failed to fetch user')
   ```

   Into:

   ```js
   throw Object.assign(new Error('Failed to fetch user'), {
     __NEXT_ERROR_CODE: 'E123abc...',
   })
   ```

   It writes a JSON file to `packages/next/error_codes` with content:

   ```json
   {
     "error_message": "The connection to the page was unexpectedly closed, possibly due to the stop button being clicked, loss of Wi-Fi, or an unstable internet connection.",
     "file_path": "packages/next/src/client/app-index.tsx",
     "occurrence_count": 1
   }
   ```

# Use in Next.js taskfile build

The plugin operates in two modes: "check" or "generate"

In check mode, it verifies the existence of required files without compilation, failing if any are missing.
In generate mode, it creates error code files in `packages/next/error_codes`.

Generate mode is exclusively used in CI environments. The CI build will fail if any error codes are uncommitted.

# Recompiling the WASM plugin after changes

After modifying the WASM plugin, recompile it using this script:

```
#!/usr/bin/env bash
set -e
NEXT_JS_ROOT="/Users/judegao/repos/next.js"
cd "$NEXT_JS_ROOT/crates/next-error-code-swc-plugin"
CARGO_PROFILE_RELEASE_STRIP=true CARGO_PROFILE_RELEASE_LTO=true cargo build --target wasm32-wasip1 --release
mv "$NEXT_JS_ROOT/target/wasm32-wasip1/release/next_error_code_swc_plugin.wasm" "$NEXT_JS_ROOT/packages/next/"
echo "✨ Successfully built and moved WASM plugin! 🚀"
```

Make sure to commit the WASM file to the repo.

# Analytics Ingestion

Error codes need to be ingested into our analytics platform for monitoring and analysis. Here's how the process works:

1. Extract Git Commit Hash
   Given an error code like `E2ab666d70ae3cc620277dcf822`, the first 10 characters (`2ab666d70a`) represent the Git commit hash where the error was introduced.

2. Locate Error Details

   - Use the GitHub API to access the codebase at the identified commit hash
   - Find the corresponding JSON file in `packages/next/error_codes`
   - Example path: `packages/next/error_codes/e3cc620277dcf822.json`

3. Parse Error Metadata
   The JSON file contains critical error information:

   ```json
   {
     "error_message": "The connection to the page was unexpectedly closed, possibly due to the stop button being clicked, loss of Wi-Fi, or an unstable internet connection.",
     "file_path": "packages/next/src/client/app-index.tsx",
     "occurrence_count": 1
   }
   ```

4. Data Pipeline Integration
   The extracted error metadata is fed into our data pipeline for:
   - Error frequency analysis
   - User experience insights

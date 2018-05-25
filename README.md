# Place
Shared editable canvas.

## Usage Notes
- The web interface will be accessible at `localhost:8000`
- The working directory should be the repository root, as `place` will use files in the `static`.
- `place` will load/create and frequently write to `canvas.place`
- A `logins.json` with entries `{ "name": { "salt": "...", "digest": "..." }, ... }` can be added to allow logging in to the console. The digest should be SHA256(password + salt) in hex.

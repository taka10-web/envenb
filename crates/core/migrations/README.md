# Migrations

**Never edit a migration that has shipped — not even a comment.**

SQLx records a checksum of every applied migration in `_sqlx_migrations`. Change
so much as one character and every existing vault refuses to open:

```
migration error: migration 1 was previously applied but has been modified
```

`0001_initial.sql` still says "EnvFish" in its first line for exactly this
reason. It was written before the rename and must stay byte-for-byte as it
shipped. The same goes for `0002` and `0003`.

Schema changes go in a **new numbered file**. `migration_checksums_never_change`
in `src/db.rs` pins the checksums so an accidental edit fails in CI rather than
on a user's machine.

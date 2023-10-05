# rembackup

A super simple yet fast backup solution, designed with slow connections in mind.

## How it works

Rembackup uses 3 directories: `source`, `index`, and `target`.

```sh
rembackup $SOURCE $INDEX $TARGET
```

In *Step 1*, Rembackup recursively walks the `source` directory, comparing all entries with `index`.
It then shows a list of changes that would make `target` contain the same files as `source`.

If you accept, it will then move on to *Step 2* and apply these changes.

If you didn't get any warnings, `target` is now a backup of `source`.

If you *did* get one or more warnings - don't worry!
You can just rerun the backup and the failed operations will be retried.

## What makes it special

If you want to back up your data to an external disk, you probably bought a large HDD.
If you want a remote backup, you may want to self-host something.
In both of these situations, the filesystem containing `target` is horribly slow.

If a backup tool tries to compare `source` to `target` to figure out which files have changed,
it will always be affected by this slowness - even when working on unchanged files.

Rembackup only performs read operations on `source` and `index` in *Step 1*.
Because of this, it can be surprisingly fast even when backing up large disks.

In *Step 2*, where files are actually being copied to `target`, the slowness will still be noticeable,
but since only modified files are being copied, this usually takes a somewhat reasonable amount of time.

## Usage

To create a backup of your home directory `~` to `/mnt/backup`:

```sh
rembackup ~ ~/index /mnt/backup
```

Note: `index` (`~/index`) doesn't need to be a subdirectory of `source` (`~`), but if it is, it will not be part of the backup to avoid problems.
Note 2: `~/index` and `/mnt/backup` don't need to exist yet - they will be created if their parent directories exist.

If this is the first backup, you can try to maximize the speed of `/mnt/backup`.
If you want remote backups, you should probably connect the server's disk directly to your computer.
The backups after the initial one will be a lot faster, so you can switch to remote backups after this.

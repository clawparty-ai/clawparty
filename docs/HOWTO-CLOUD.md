# Cloud Guide

The builtin zero-trust cloud app makes it easy to share files between devices and users. Unlike centralized cloud services such as *Apple iCloud*, your files are stored distributively on multiple endpoint devices that can be located in various private local networks while still allowing you to access them remotely and securely over the Internet.

To start using the Cloud app, you need a *local directory* acting as a *mirror* of the entire mesh filesystem. Each endpoint can have its own location for the local directory, but the content in it will be consistent on every endpoint by *manual downloading* or *auto-mirroring*.

The default local directory is `~/ztmCloud`. You can change to a different location for the current local endpoint by:

```sh
ztm cloud config --local-dir /path/to/my/local/mirror
```

> If you are configuring a remote endpoint, put `ep my-remote-ep` before `cloud config` to specify that.

To see what files are already stored in the cloud, type `ztm cloud ls <path>`. For example:

```sh
ztm cloud ls /users/root
NAME       STATE   SIZE        DATE                      SOURCES  SHARED
video.mp4  new     2055491257  Thu Aug 22 20:14:37 2024  0        -
dummy.txt  synced  12          Tue Sep 10 11:46:41 2024  2        -
shared/    -       -           -                         -        All Users
```

The *STATE* column shows the current status of a file in the *local directory* compared to what has been stored on the mesh. It can be one of:

- *Synced*: The local file is identical to what's on the mesh
- *New*: The local file is new to the mesh, in other words, the mesh doesn't have the file yet
- *Changed*: The mesh does have the file but is older than the local file
- *Missing*: The mesh has the file but the local directory hasn't
- *Outdated*: The mesh has a newer version of the file than in the local directory

The *SOURCES* column tells us how many endpoints have that file in their local directories. In other words, how many copies we have across the entire mesh.

To put a local file up to the mesh, which means to make a file *visible* to other endpoints, use `upload` command:

```sh
ztm cloud upload /users/root/video.mp4
```

If the file is *new* to the mesh, after uploading it, the *SOURCES* column of the file becomes 1, since now we have only one endpoint hosting the file right now, which is just the current local endpoint.

```sh
ztm cloud ls /users/root/video.mp4
NAME       STATE   SIZE        DATE                      SOURCES  SHARED
video.mp4  synced  2055491257  Thu Aug 22 20:14:37 2024  1        -
```

To download the file on a different endpoint, use `download` command on that endpoint:

```sh
# On a second endpoint
ztm cloud download /users/root/video.mp4
```

The file will be downloaded from the first endpoint to the local directory of the second endpoint, and the *SOURCES* column will increase to 2, since now we have 2 endpoints that have the same file in their local directories.

If you'd like the second endpoint to automatically download any new uploads to the mesh, you can set up an *auto-mirror* for directory `/users/root`:

```sh
# On the second endpoint
ztm cloud config --add-mirror /users/root
```

After that, the second endpoint will be working as a backup for all files under `/users/root`.

Although files can be easily shared across endpoints just like that, by default, they are only accessible by the owner. In the previous examples, all files are under `/users/root` and owned by `root` user, and thus, inaccessible to other users.

To share a file or directory to all other users, use `share` command:

```sh
ztm cloud share /users/root/shared --set-all readonly
```

To share a file or directory to a specific user, use `--set-readonly` option with the `share` command:

```sh
ztm cloud share /users/root/video.mp4 --set-readonly guest
```

For more options of the `share` command, type:

```sh
ztm cloud help share
```

You can also see detailed info about all other commands by typing `ztm cloud help [command]`.

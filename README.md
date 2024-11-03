# TODO: DEPS

# Blog web app

This simple app provides a platform to share blogposts.
Each blogpost consists of:

* main text
* date of the publication
* optional image in the .png format.
* user name
* optional user avatar in the .png format

The posts are persisted in a SQLite database in a file.

The images are sanitized using [image.rs](https://docs.rs/image/latest/image/).

## Run instructions

### Run from cloned repository

```shell
docker compose up
```

### Run from dockerhub

```shell
docker run -p 3000:3000 mbienkowsk/rust-blogspot-web-app:latest
```

and navigate to `localhost:3000` in your browser to view the app.

##### set env #####
set shell := ["bash", "-uc"]
set dotenv-load := true


##### variables ######
app_name        := `cargo metadata --format-version=1 --no-deps | jq .packages[0].name | sed -e 's/"//g'`
app_version     := "v" + `cargo metadata --format-version=1 --no-deps | jq .packages[0].version | sed -e 's/"//g'`
image_name     := "mk10969/" + app_name

##### commands ######
# app name
app:
    @echo {{ app_name }}

# app version
version:
    @echo {{ app_version }}

# build container image
docker-build:
    docker build ./ -t {{ image_name }}:{{ app_version }}

# run docker container
docker-run: docker-build
    docker run -d \
        --env-file .env \
        -p 9000:9000 \
        {{ image_name }}:{{ app_version }}

# rm docker container
docker-stop:
    docker ps \
        | grep "{{ image_name }}:{{ app_version }}" \
        | cut -d ' ' -f 1 \
        | xargs docker rm -f

# git tag
tag:
    git tag -a {{ app_version }} -m 'version up'

# push tag
push:
    git push origin {{ app_version }}

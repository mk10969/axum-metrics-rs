##### set env #####
set shell := ["bash", "-uc"]
set dotenv-load := true


##### variables ######
app_name        := `cargo metadata --format-version=1 --no-deps | jq .packages[0].name | sed -e 's/"//g'`
app_version     := "v" + `cargo metadata --format-version=1 --no-deps | jq .packages[0].version | sed -e 's/"//g'`
app_image_name     := "mk10969/" + app_name

##### commands ######
# app name
app:
    @echo {{ app_name }}

# app version
version:
    @echo {{ app_version }}

# rm docker images
docker-rmi:
    @echo "========== remove docker image =========="
    docker images \
        | grep "{{ app_image_name }}" \
        | awk -F " " '{print $3}' \
        | xargs -I '{}' docker rmi -f '{}'

# build container image
docker-build: docker-rmi
    @echo "========== docker build =========="
    docker build ./ -t {{ app_image_name }}:{{ app_version }}

# run docker container
docker-run: docker-build
    docker run -d \
        --env-file .env \
        -p 9000:9000 \
        {{ app_image_name }}:{{ app_version }}

# rm docker container
docker-stop:
    docker ps \
        | grep "{{ app_image_name }}:{{ app_version }}" \
        | cut -d ' ' -f 1 \
        | xargs docker rm -f

# latest tag
docker-tag: docker-build
    @echo "========== stamp latest tag =========="
    docker images \
        | grep "{{ app_image_name }}" \
        | grep "{{ app_version }}" \
        | awk -F " " '{print $3}' \
        | xargs -I '{}' docker tag '{}' {{ app_image_name }}:latest

# docker push images
docker-push: docker-tag
    @echo "========== app version =========="
    docker push {{ app_image_name }}:{{ app_version }}
    @echo "========== latest version =========="
    docker push {{ app_image_name }}:latest

# git tag
tag:
    git tag -a {{ app_version }} -m 'version up'

# push tag
push:
    git push origin {{ app_version }}

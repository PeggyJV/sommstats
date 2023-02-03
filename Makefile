.DEFAULT_GOAL := build_image

build_image:
	docker build -t sommstats:prebuilt -f Dockerfile .

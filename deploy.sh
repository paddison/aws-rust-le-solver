#!/bin/bash

# FILE=target/lambda/gauss_lambda/bootstrap.zip
# if ! test -f "$FILE"; then
# 	zip -j ./target/lambda/gauss_lambda/bootstrap.zip ./target/lambda/gauss_lambda/bootstrap
# fi
cargo lambda build --release --target x86_64-unknown-linux-gnu --output-format zip

aws cloudformation package --template-file template.yml \
                           --s3-bucket rust-deployment \
                           --output-template-file out.yml


aws cloudformation deploy --template-file out.yml \
                          --stack-name my-stack \
                          --capabilities CAPABILITY_IAM
use aws_sdk_s3::output::GetObjectOutput;
use aws_sdk_s3::output::PutObjectOutput;
use log::{ debug, error, info };
use serde::{ Deserialize, Serialize };
use lambda_runtime::{ Error, LambdaEvent, service_fn };
use serde_json::Value;
use gauss::helpers::*;
use gauss::Matrix;
use aws_sdk_s3::Client;

// represents the data the Lambda will receive, must implement serde::Deserialize
// todo fit struct to match lambda trigger event
#[derive(Deserialize, Debug)]
struct Request {
    pub body: String,
}

// Data returned on success
#[derive(Debug, Serialize)]
struct SuccessResponse {
    pub body: String,
}

#[derive(Debug, Serialize)]
struct FailureResponse {
    pub body: String,
}

// Implement Display for the Failure response so that we can then implement Error.
impl std::fmt::Display for FailureResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.body)
    }
}

// Implement Error for the FailureResponse so that we can `?` the Response
// returned by `lambda_runtime::run(func).await` in `fn main`.
impl std::error::Error for FailureResponse {} // use blanket implementation

type Response = Result<SuccessResponse, FailureResponse>;

#[tokio::main]
async fn main() -> Result<(), Error> {
    // You can view the logs emitted by your app in Amazon CloudWatch.
    tracing_subscriber::fmt::init();
    debug!("logger has been set up");

    let func = service_fn(matrix_handler);
    lambda_runtime::run(func).await?;

    Ok(())
}

async fn matrix_handler(req: LambdaEvent<Value>) -> Response {
    info!("handling a request...");

    // setting up environment
    let (event, _) = req.into_parts();
    let config = aws_config::load_from_env().await;
    let client = aws_sdk_s3::Client::new(&config);
    let read_bucket = std::env::var("READ_BUCKET")
        .expect("A READ_BUCKET must be set in this apps's Lambda environment variables.");
    let write_bucket = std::env::var("WRITE_BUCKET")
        .expect("A WRITE_BUCKET must be set in this apps's Lambda environment variables.");

    // get name of the file that was uploaded
    let key = event["Records"][0]["s3"]["object"]["key"].as_str()
        .expect("Couldn't find 'key' field in request");

    // Get file from bucket
    info!("Name of uploaded file: {}", key);
    let resp = get_from_bucket(&client, &read_bucket, key).await?;
    
    debug!("Data body: {:?}", resp);
 
    // read in file content
    info!("reading file content to string...");
    let content = map_output_to_string(resp).await?;

    // try to parse file content to matrix and vector
    info!("parsing string to linear equation...");
    let (mut matrix, vector) = string_to_matrix(content)?;

    // calculate the result of the linear equation
    info!("calculating solution...");
    let result = match matrix.solve(vector) {
        Some(solution) => format!("{:?}", solution),
        None => String::from("No solution found"),
    };

    // store the result in a different bucket
    info!("storing solution in {}...", write_bucket);
    let _ = store_in_bucket(&client, &write_bucket, key, &result).await?;

    info!("Done");
    Ok(SuccessResponse {
        body : format!("Result was: {}, it was stored in bucket {}/{}", result, write_bucket, key),
    })
}

async fn get_from_bucket(client: &Client, bucket: &str, key: &str) -> Result<GetObjectOutput, FailureResponse> {
    client
    .get_object()
    .bucket(bucket)
    .key(key)
    .send()
    .await
    .map_err(|err| {
        // In case of failure, log a detailed error to CloudWatch.
        error!(
            "Couldn't find file: '{}' to S3 with error: {}",
            key, err
        );
        FailureResponse {
            body: "The lambda encountered an error and your calculation wasn't saved".to_owned(),
        }
    })
}

async fn store_in_bucket(client: &Client, bucket: &str, key: &str, result: &str) -> Result<PutObjectOutput, FailureResponse> {
    client
        .put_object()
        .bucket(bucket)
        .body(result.as_bytes().to_owned().into())
        .key(key)
        .content_type("text/plain")
        .send()
        .await
        .map_err(|err| {
            // In case of failure, log a detailed error to CloudWatch.
            error!(
                "failed to upload file '{}' to S3 with error: {}",
                key, err
            );
            FailureResponse {
                body: "The lambda encountered an error and your message was not saved".to_owned(),
            }
        })
}

async fn map_output_to_string(output: GetObjectOutput) -> Result<String, FailureResponse> {
    let data = match output.body.collect().await {
      Ok(bytes) => bytes.into_bytes(),
      Err(e) => return Err(FailureResponse {
        body: e.to_string(),
    })
    };

    let mut data_vec = vec![];
    for byte in data {
        data_vec.push(byte);
    }

    match String::from_utf8(data_vec) {
        Ok(string) => Ok(string),
        Err(e) => return Err(FailureResponse {
            body: e.to_string(),
        })
    }

}

fn string_to_matrix(buffer: String) -> Result<(Matrix, Vec<f64>), FailureResponse>{
    let mut dimension = 0;
    let mut raw_matrix: Vec<f64> = vec![];

    for line in buffer.split("\n") {
        if dimension == 0 {
            dimension = match parse_row_input(&mut raw_matrix, line, 0) {
                Ok(dimension) => dimension,
                Err(e) => return Err(FailureResponse {
                    body: e.to_owned(),
                })
            }
        } else {
            match parse_row_input(&mut raw_matrix, line, dimension) {
                Ok(_) => (),
                Err(e) => return Err(FailureResponse {
                    body: e.to_owned(),
                })
            }
        }   
    }

    Ok((Matrix::from(&raw_matrix[0..dimension * dimension]), Vec::from(&raw_matrix[dimension * dimension..])))
}
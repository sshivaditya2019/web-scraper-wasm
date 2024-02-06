# WASM Based API

A API based web scraper, written in Rust and compiled to WASM. Then deployed to a serverless function on Cloudflare Workers. This uses github OAuth to authenticate the user and issue client id and client secret.
The user can then use their client id, client secret and uuid, to generate a JWT token. This token can be used to access the API. The API can be used to scrape the web and return the data in JSON format.

## Deployed
- [WASM API](https://api.shivadityas.com)

## Usage
- Navigate to the deployed root and go the `/oauth` endpoint.
- Click on the `Authorize` button and login with your github account.
- After logging in, you will be redirected to the `/callback/github` endpoint.
- Copy the `client_id`, `client_secret` and `uuid` from the URL.
- Use the `/authorize` endpoint to generate a JWT token.
- Use the token to access the `/api/scrape` endpoint.

## Endpoints

### /authorize
- Method: POST
- Description: This endpoint is used to authenticate the user and issue client id and client secret.
- Request Body:
  - `client_id`: String
  - `client_secret`: String
  - `uuid`: String
- Response:
    - `Token`: String
    - `Type`: String

### /api/scrape
- Method: GET
- Description: This endpoint is used to scrape the web and return the data in JSON format.
- Request Headers:
  - `Authorization`: String
- Response:
  - `Result`: JSON
  Each article will have the following fields:
    - `Title`: String
    - `Link`: String
    - `Time`: String
    - `Author`: String
    - `Source Link`: String
    - `Source Name`: String
    - `Image Link`: String (Optional)

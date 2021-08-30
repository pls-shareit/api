# API

The services exposes the following routes:

## `POST /`

This creates a new share with a random name. The body, `Share-Type` header, and
type-specific headers should be set as follows:

| `Share-Type` header | Body                    | Other headers        |
|---------------------|-------------------------|----------------------|
| `link`              | A full URL              | -                    |
| `file`              | File contents           | `Content-Type`       |
| `paste`             | Paste contents as UTF-8 | `Share-Highlighting` |

Additionally, an `Expire-After` header can be set to specify the number of
seconds that the share should be kept for. If you are using a password to
access the service, you should set it in the `Authorization` header.

This endpoint returns the link to the newly created share in the body of the
response, and a token for managing the share in the `Share-Token` header.

## `POST /<name>`

Like `POST /`, but specify the name of the share to create. It will return a
`409` error if the name is already taken.

## `GET /<name>`

Get the contents of a share. For a link share, this will return an HTTP
temporary redirect to the link. For a file share, this will return the file
contents, with the `Content-Type` set appropriately. For a paste share, this
will return the paste contents, with the `Share-Highlighting` header set.

If a frontend is configured, adding `?v` to the end of the URL will return the
`share.html` file of the frontend instead. It is up to the frontend to retrieve
the contents of the share. If a frontend is not configured, adding `?v` will
not change the response.

## `DELETE /<name>`

Delete a share you created. The `Authorization` header must be set to the share
token. This endpoint returns a `204` response if successful, a `401` response
if the token is invalid, or a `403` response if updating shares is disabled.

## `UPDATE /<name>`

Update a share you created. The `Authorization` header must be set as with the
`DELETE` endpoint.

The body of the request may be the same as the `POST` endpoint, or empty to
keep the contents of the share the same. The `Expire-After` header can be set
as with the `POST` endpoint to update or extend the expiry time of the share -
it will be set relative to the time of the request.

For file and paste shares respectively, the `Content-Type` and
`Share-Highlighting` headers can be set to update the share metadata.

## `GET /`

Returns the `index.html` file of the frontend if enabled, or a `404` error
otherwise.

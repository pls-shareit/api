# API

## Authentication

The service supports the use of passwords for certain endpoints. Depending on
the configuration of the server, a password may be mandatory to use certain
features.

To use provide a password to an endpoint, set the `Authorization` header to
`Password <password goes here>`. For example:

```
Authorization: Password mysupersecretpassword
```

Depending again on the configuration of the server and the password you use,
the endpoint for creating a share may return a token, which can be used to
update or delete the share. To use this token, set the `Authorization` header
to `Token <token goes here>` when creating or deleting the share. For example:

```
Authorization: Token NO8y1YeaCmMlSiy77sp7nMhsbNNoELcHYl9v9uhGwk0
```

(This is an example, a real token will be longer.)

Note that if you have a password with the "update_any" permission, you can use
that instead of the share token, with the `Password` authorisation method.

Endpoints that pay attention to authentication will return a 400 error if
authentication is badly formatted, a 401 error if authentication is understood
but not acknowledged (eg. if the password is wrong), or a 403 error if the
authentication is correctly formatted and recognised, but the authorisation is
not sufficient for the action requested.

## Endpoints

The services exposes the following routes:

### `GET /meta/abilities`

This endpoint can be used to retrieve the enable API capabilities that are
available for the authenticated user. For accurate results, you should pass the
same `Authorization` header as you would use to create a share.

This endpoint returns a JSON object with the following boolean fields:

| Field          | Description                                                  |
| -------------- | ------------------------------------------------------------ |
| `login`        | Whether a password may set for more abilities.               |
| `create_file`  | Whether a file share may be created.                         |
| `create_link`  | Whether a link share may be created.                         |
| `create_paste` | Whether a paste share may be created.                        |
| `update_own`   | Whether a share token will be returned when creating shares. |
| `update_any`   | Whether the password can be used to update any share.        |

It will also contain:

- A `custom_names` field, either `null` meaning that you cannot create custom
  names, or an object containing the `min_length` and `max_length` for custom
  names as integers.

- `mime_types_whitelist` and `mime_types_blacklist` fields, corresponding
  directly to
  [the relevant config options](configuration.md#allowed_mime_types-and-disallowed_mime_types).

- A `link_schemes` field, corresponding directly to
  [the `allowed_link_schemes` config option](configuration.md#allowed_link_schemes).

- A `highlighting_languages` field, corresponding directly to
  [the `highlighting_languages` config option](configuration.md#highlighting_languages).

- A `max_expiry_time` field, corresponding to the
  [`max_expiry_time` config option](configuration.md#max_expiry_time), given
  as an integer in seconds.

This endpoint will return a 401 error if an unknown password is used.

### `POST /`

This creates a new share with a random name. The body, `Share-Type` header, and
type-specific headers should be set as follows:

| `Share-Type` header | Body                    | Other headers        |
|---------------------|-------------------------|----------------------|
| `link`              | A full URL              | -                    |
| `file`              | File contents           | `Content-Type`       |
| `paste`             | Paste contents as UTF-8 | `Share-Highlighting` |

Additionally, an `Expire-After` header can be set to specify the number of
seconds that the share should be kept for.

A password can be set as described in [**Authentication**](#authentication).

This endpoint returns the link to the newly created share in the body of the
response.

If the authenticated user is allowed to update and delete their own shares, a
token for managing this share will be returned in the `Share-Token` header.

### `POST /<name>`

Like `POST /`, but specify the name of the share to create. It will return a
`409` error if the name is already taken.

### `GET /<name>`

Get the contents of a share. For a link share, this will return an HTTP
temporary redirect to the link. For a file share, this will return the file
contents, with the `Content-Type` set appropriately. For a paste share, this
will return the paste contents, with the `Share-Highlighting` header set.

The `Share-Type` header will also be set on the response, to one of `link`,
`file`, or `paste`.

If the `Accept-Redirect` header on the request is set to `no`, the server will
give exactly the same response, but use the `200` status code instead of `307`.
For non-link shares, it will always use `200` for successful requests.

This is to allow web frontends to resolve link shares, since browsers do not
allow scripts to get the resolved URL of an HTTP redirect (see
[the spec](https://fetch.spec.whatwg.org/#atomic-http-redirect-handling)).

### `DELETE /<name>`

Delete a share you created. This endpoint returns a `204` response if
successful.

A password or share token should be set as described in
[**Authentication**](#authentication).

### `UPDATE /<name>`

Update a share you created.

The body of the request may be the same as the `POST` endpoint, or empty to
keep the contents of the share the same. The `Expire-After` header can be set
as with the `POST` endpoint to update or extend the expiry time of the share -
it will be set relative to the time of the request.

For file and paste shares respectively, the `Content-Type` and
`Share-Highlighting` headers can be set to update the share metadata.

A password or share token should be set as described in
[**Authentication**](#authentication).

This endpoint will give the same response as the `GET /<name>` endpoint,
including handling the `Accept-Redirect` header.

### `GET /`

Returns the `index.html` file of the frontend if enabled, or a `404` error
otherwise. Arbitrary query parameters may be passed, which will be ignored
(but may be useful for the frontend).

table! {
    shares (name) {
        name -> Varchar,
        expiry -> Nullable<Timestamp>,
        token -> Varchar,
        kind -> Int2,
        link -> Nullable<Varchar>,
        language -> Nullable<Varchar>,
        mime_type -> Nullable<Varchar>,
    }
}

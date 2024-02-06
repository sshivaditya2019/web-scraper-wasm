

pub fn home_page() -> String {
    let html = r#"
    <!DOCTYPE html>
    <html>
        <head>
            <meta charset="utf-8">
            <title>Home</title>
        </head>
        <body>
            <h1>Home</h1>
            <p>Welcome to the home page!</p>
        </body>
    </html>
    "#;
    html.to_string()
}

pub fn oauth_home_page() -> String {
    let html = r#"
    <!DOCTYPE html>
    <html>
        <head>
            <meta charset="utf-8">
            <title>OAuth</title>
        </head>
        <body>
            <h1>OAuth</h1>
            <p>Welcome to the OAuth home page!</p>
            <a href="/oauth/github">Login with GitHub</a>
        </body>
    </html>
    "#;
    html.to_string()
}
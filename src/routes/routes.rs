use worker::*;




//Write a route that matches the path "/hello" and returns a Response with the body "Hello from Workers!".
// Path: src/routes/routes.rs
pub fn hello(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    Response::ok("Hello from Workers!")
}



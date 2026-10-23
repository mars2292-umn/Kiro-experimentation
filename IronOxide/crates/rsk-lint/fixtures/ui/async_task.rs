//@ scope: application
//@ PR-09: the async forms a Task body may not contain (R20).
async fn job() {}

fn task() {
    let block = async { 1 };
    let fut = job();
    let _ = fut.await;
    let _c = async || 2;
    let _ = block;
}

## Rust Development Pitfalls

Here is where I record bugs, errors, and pitfalls during the development.


### Double Return Casues Lifetime Conflict

Code:

```
async fn handle<'a>(&mut self, sync_task: &'a mut SyncTask<'a>) -> Result<&'a mut SyncTask<'a>, Box<dyn Error>> {
    // async fn handle<'a>(&mut self, sync_task: &'a mut SyncTask<'a>) -> Result<(), Box<dyn Error>> {
        self.state = WorkerState::Working;
        sync_task.start();
        match sync_task.spec().request_method() {
            RequestMethod::Get => {
                // DO SOMETHING
  
                match resp {
                    Ok(resp) => {
                        info!("status: {}", resp.status());
                        let json: Value = resp.json().await?;
                        sync_task.set_result(Some(json))
                                 .set_end_time(Some(chrono::offset::Local::now()))
                                 .finished();
                        // Should return here
                        // return Ok(sync_task);
                    },
                    Err(error) => {
                        error!("error: {}", error);
                        sync_task.set_end_time(Some(chrono::offset::Local::now()))
                                 .set_result_message(Some(error.to_string()))
                                 .failed();
                        // Should return here
                    }
                }
            },
            RequestMethod::Post => {
                // similar logic
            },
  
        }
        // Should not return here
        // return Ok(sync_task);
    }
```

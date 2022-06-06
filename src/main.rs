use std::{thread::{self}, sync::{mpsc::{self, Sender, Receiver, TryRecvError, SendError}, Arc, Mutex, Weak}, time::Duration, task::Waker};
use futures::{executor, Future};                
fn main(){ 
    let fut = main_async();
    executor::block_on(fut);
}

fn mpsc_channel_async<T>() -> (MyMpscSender<T>, MpscReceiverFuture<T>){
    let (s,r) = mpsc::channel();
    let waker = Arc::new(Mutex::new(None));
    let w = waker.clone();
    let sender = MyMpscSender{sender: s, waker: w};
    let w = Arc::downgrade(&waker);
    let receiver = MpscReceiverFuture{receiver: r, waker_weak_p: w};
    (sender, receiver)
}


async fn main_async(){
    let (sender, receiver) = mpsc_channel_async();
    let thread_handle = std::thread::spawn(move || {              //this thread is just here so the f1 function gets blocked by something and can later resume
        wait_send_function(sender);
    });
    let f1 = f1(receiver);
    let f2 = f("Starting f2!");
    let f3 = f("Starting f3!");

    futures::join!(f1, f2, f3);
    thread_handle.join().unwrap();
}

fn wait_send_function(sender: MyMpscSender<i32>){
    thread::sleep(Duration::from_millis(5000));
    sender.send(1234).unwrap();
}


async fn f1(receiver: MpscReceiverFuture<i32>){
    println!("starting f1");
    let new_nmbr = receiver.await.unwrap();               
    println!("F1: Received nmbr is: {}", new_nmbr);
}


async fn f(o: &str){
    println!("{}", o);
}



struct MyMpscSender<T>{                //this should be privated, so it can't actually be cloned or accessed?
    sender: Sender<T>,           
    waker: Arc<Mutex<Option<Waker>>>,
}

impl <T>MyMpscSender<T>{
    pub fn send(&self, data: T )-> Result<(), SendError<T>>{
        let ret = self.sender.send(data);
        let mut lock = self.waker.lock().unwrap();
        if let Some(waker) = lock.as_mut(){
            waker.wake_by_ref();
        }
        ret
    }
}

impl <T>Clone for MyMpscSender<T>{
    fn clone(&self) -> Self {
        Self { sender: self.sender.clone(), waker: self.waker.clone() }
    }
}


struct MpscReceiverFuture<T>{          
    receiver: Receiver<T>,
    waker_weak_p: Weak<Mutex<Option<Waker>>>,                  
}


impl <T>Future for MpscReceiverFuture<T> {
    type Output = Result<T, TryRecvError>;

    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
        println!("Polling MpscReceiverFuture!");
        let waker = cx.waker().clone();
        if let Some(mutex) = self.waker_weak_p.upgrade(){
            let mut lock = mutex.lock().unwrap();
            *lock = Some(waker);
        }
        match self.receiver.try_recv(){
            Ok(ret) => {println!("... MpscReceiverFuture is done!");std::task::Poll::Ready(Ok(ret))},
            Err(err) => match err {
                TryRecvError::Empty => {println!("... MpscReceiverFuture is pending!"); std::task::Poll::Pending},
                TryRecvError::Disconnected => std::task::Poll::Ready(Err(err)),
            },
        }
    }
}

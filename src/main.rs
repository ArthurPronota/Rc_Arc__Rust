use std::sync::Arc ;
use std::thread ;

fn main() {
    let v = Arc::new(vec![1, 2, 3, 4, 5]) ;

    thread::spawn({
        let v = Arc::clone(&v) ;
        move || {
            println!("{v:?}") ;
        }
    })
    .join()
    .unwrap()   // Out: [1, 2, 3, 4, 5]
     ;
}

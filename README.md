# kxkdb

Kdb+ interface for the Rust programming language.

The interface comprises two features:
- IPC: Connecting Rust and kdb+ processes via IPC
- API: Embedding Rust code inside kdb+ processes

## Documentation

Documentation for this interface can be found at https://docs.rs/kxkdb/.

## Kdbplus

The kxkdb interface is forked from the excellent [kdbplus](https://crates.io/crates/kdbplus) interface, developed by [diamondrod](https://github.com/diamondrod).

## IPC

The IPC feature enables [qipc](https://code.kx.com/q/basics/ipc/) communication between Rust and kdb+.

Connectivity is via TCP or Unix Domain Sockets, with support for both [compression](https://code.kx.com/q/basics/ipc/#compression) and [TLS encryption](https://code.kx.com/q/kb/ssl/) of messages.

Connection and listener methods are provided, enabling development of both
- Rust IPC clients of kdb+ server processes
- Rust IPC servers of kdb+ client processes

### Installation

Add `kxkdb` as a dependency, with feature `ipc`.
You may also want to add an asynchronous runtime such as [Tokio](https://tokio.rs).

e.g.
```toml
[dependencies]
kxkdb = { version = "0.0", features = ["ipc"] }
tokio = { version = "1.24", features = ["full"] }
```

### Examples

#### Client

```rust
use kxkdb::ipc::*;
use kxkdb::qattribute;

#[tokio::main]
async fn main() -> Result<()> {
    let mut socket;  // socket connection to kdb+ process
    let mut result;  // result of sync query to kdb+ process
    let mut message; // compound list containing message

    // connect via UDS to local kdb+ process listening on port 4321
    socket = QStream::connect(ConnectionMethod::UDS, "", 4321_u16, "").await?;

    // confirm connection type
    println!("Connection type: {}", socket.get_connection_type());

    // synchronously query kdb+ process using string
    result = socket.send_sync_message(&"sum 1+til 100").await?;
    println!("result1: {}", result);

    // asynchronously define function in kdb+ process
    socket.send_async_message(&"add_one:{x+1}").await?;
   
    // synchronously call function (correctly)
    result = socket.send_sync_message(&"add_one 41").await?;
    println!("result2: {}", result);
   
    // synchronously call function (incorrectly)
    result = socket.send_sync_message(&"add_one`41").await?;
    println!("result3: {}", result);

    // synchronously query kdb+ process using compound list
    message = K::new_compound_list(vec![K::new_symbol(String::from("add_one")), K::new_long(100)]);
    result = socket.send_sync_message(&message).await?;
    println!("result4: {}", result);
    
    // asynchronously call show function in kdb+ process
    message = K::new_compound_list(vec![K::new_string(String::from("show"), qattribute::NONE), K::new_symbol(String::from("hello from rust"))]);
    socket.send_async_message(&message).await?;

    // close socket
    socket.shutdown().await?;

    Ok(())
}
```

#### Server

Setup a credentials file containing usernames and (SHA-1 encrypted) passwords.

e.g.
```
$ cat userpass.txt
fred:e962cde7053eed120f928cd18e58ebd31be77543
homer:df43ad44d44e898f8f4e6ed91e6952bfce573e12
```
Note: Hashed passwords can be generated in q using `.Q.sha1`.

Store the path of this file in environment variable KDBPLUS_ACCOUNT_FILE.

e.g.
```
$ export KDBPLUS_ACCOUNT_FILE=`pwd`/userpass.txt
```

The following code will establish a Rust server process, listening on port 4321.
```rust
use kxkdb::ipc::*;

#[tokio::main]
async fn main() -> Result<()> {
    let mut socket;   // socket connection to kdb+ process

    // listen for incoming TCP connections on port 4321
    socket = QStream::accept(ConnectionMethod::TCP, "127.0.0.1", 4321).await?;

    // when a connection is established, synchronously send a message to the client
    let response = socket.send_sync_message(&"0N!string `Hello").await?;
    println!("result: {}", response);

    // close socket
    socket.shutdown().await?;

    Ok(())
}

```

A kdb+ client can then connect using the correct credentials.

e.g.
```q
q)hopen`:127.0.0.1:4321:fred:flintstone;
"Hello"
```

### Type Mapping

The following table displays the input types used to construct different q types (implemented as `K` objects).

| q                | Rust                                      |
|------------------|-------------------------------------------|
| `boolean`        | `bool`                                    |
| `guid`           | `[u8; 16]`                                |
| `byte`           | `u8`                                      |
| `short`          | `i16`                                     |
| `int`            | `i32`                                     |
| `long`           | `i64`                                     |
| `real`           | `f32`                                     |
| `float`          | `f64`                                     |
| `char`           | `char`                                    |
| `symbol`         | `String`                                  |
| `timestamp`      | `chrono::DateTime<Utc>`                   |
| `month`          | `chrono::NaiveDate`                       |
| `date`           | `chrono::NaiveDate`                       |
| `datetime`       | `chrono::DateTime<Utc>`                   |
| `timespan`       | `chrono::Duration`                        |
| `minute`         | `chrono::Duration`                        |
| `second`         | `chrono::Duration`                        |
| `time`           | `chrono::Duration`                        |
| `list`           | `Vec<T>` (`T` a corresponding type above) |
| `compound list`  | `Vec<K>`                                  |
| `table`          | `Vec<K>`                                  |
| `dictionary`     | `Vec<K>`                                  |
| `generic null`   | `()`                                      |

Note: The input type can differ from the inner type. For example, timestamp has an input type of `chrono::DateTime<Utc>` but the inner type is `i64`, denoting an elapsed time in nanoseconds since `2000.01.01D00:00:00`.

### Environment Variables

#### KDBPLUS_ACCOUNT_FILE

Path to a credential file, used by a Rust server to manage access from kdb+ clients.

Contains a user name and SHA-1 hashed password on each line, delimited by `':'`.

#### KDBPLUS_TLS_KEY_FILE

The path to a pkcs12 file used for TLS connections.

#### KDBPLUS_TLS_KEY_FILE_SECRET

The password for the above pkcs12 file.

#### QUDSPATH

Defines the (real or abstract) path used for [Unix Domain Sockets](https://code.kx.com/q/basics/listening-port/#unix-domain-socket) to `$QUDSPATH/kx.[PORT]`.

n.b. If not defined, this will default to `/tmp/kx.[PORT]`

## API

The API feature enables the development of shared object libraries in Rust, which can be [dynamically loaded](https://code.kx.com/q/ref/dynamic-load/) into kdb+.

In order to avoid large `unsafe` blocks, most native C API functions are provided with a wrapper funtion and with intuitive implementation as a trait method. The exceptions are variadic functions `knk` and `k`, which are provided under `native` namespace with the other C API functions.

### Installation

Add `kxkdb` as a dependency, with feature `api`.

```toml
[dependencies]
kxkdb={version="0.0", features=["api"]}
```

### Examples

#### C API Style

```rust
use kxkdb::qtype;
use kxkdb::api::*;
use kxkdb::api::native::*;

#[no_mangle]
pub extern "C" fn create_symbol_list(_: K) -> K {
    unsafe{
        let mut list=ktn(qtype::SYMBOL_LIST as i32, 0);
        js(&mut list, ss(str_to_S!("Abraham")));
        js(&mut list, ss(str_to_S!("Isaac")));
        js(&mut list, ss(str_to_S!("Jacob")));
        js(&mut list, sn(str_to_S!("Josephine"), 6));
        list
    }
}
 
#[no_mangle]
pub extern "C" fn catchy(func: K, args: K) -> K {
    unsafe{
        let result=ee(dot(func, args));
        if (*result).qtype == qtype::ERROR{
            println!("error: {}", S_to_str((*result).value.symbol));
            // Decrement reference count of the error object
            r0(result);
            KNULL
        } else {
            result
        }
    }
}

#[no_mangle]
pub extern "C" fn dictionary_list_to_table() -> K {
    unsafe{
        let dicts = knk(3);
        let dicts_slice = dicts.as_mut_slice::<K>();
        for i in 0..3 {
            let keys = ktn(qtype::SYMBOL_LIST as i32, 2);
            let keys_slice = keys.as_mut_slice::<S>();
            keys_slice[0] = ss(str_to_S!("a"));
            keys_slice[1] = ss(str_to_S!("b"));
            let values = ktn(qtype::INT_LIST as i32, 2);
            values.as_mut_slice::<I>()[0..2].copy_from_slice(&[i*10, i*100]);
            dicts_slice[i as usize] = xD(keys, values);
        }
        // Format list of dictionary as a table.
        // ([] a: 0 10 20i; b: 0 100 200i)
        k(0, str_to_S!("{[dicts] -1 _ dicts, (::)}"), dicts, KNULL)
    } 
}
```

A kdb+ process can then dynamically load and call these functions as follows:

```q
q)summon:`libc_api_examples 2: (`create_symbol_list; 1)
q)summon[]
`Abraham`Isaac`Jacob`Joseph
q)`Abraham`Isaac`Jacob`Joseph ~ summon[]
q)catchy: `libc_api_examples 2: (`catchy; 2);
q)catchy[$; ("J"; "42")]
42
q)catchy[+; (1; `a)]
error: type
q)behold: `libc_api_examples 2: (`dictionary_list_to_table; 1);
q)behold[]
a  b  
------
0  0  
10 100
20 200
```

#### Rust Style

The examples below are written without `unsafe` code.

```rust
use kxkdb::qtype;
use kxkdb::api::*;
use kxkdb::api::native::*;

#[no_mangle]
pub extern "C" fn create_symbol_list2(_: K) -> K {
    let mut list = new_list(qtype::SYMBOL_LIST, 0);
    list.push_symbol("Abraham").unwrap();
    list.push_symbol("Isaac").unwrap();
    list.push_symbol("Jacob").unwrap();
    list.push_symbol_n("Josephine", 6).unwrap();
    list
}

#[no_mangle]
fn no_panick(func: K, args: K) -> K {
    let result = error_to_string(apply(func, args));
    if let Ok(error) = result.get_error_string() {
        println!("FYI: {}", error);
        // Decrement reference count of the error object which is no longer used.
        decrement_reference_count(result);
        KNULL
    }
    else{
        println!("success!");
        result
    }
}

#[no_mangle]
pub extern "C" fn create_table2(_: K) -> K {
    // Build keys
    let keys = new_list(qtype::SYMBOL_LIST, 2);
    let keys_slice = keys.as_mut_slice::<S>();
    keys_slice[0] = enumerate(str_to_S!("time"));
    keys_slice[1] = enumerate_n(str_to_S!("temperature_and_humidity"), 11);

    // Build values
    let values = new_list(qtype::COMPOUND_LIST, 2);
    let time = new_list(qtype::TIMESTAMP_LIST, 3);
    // 2003.10.10D02:24:19.167018272 2006.05.24D06:16:49.419710368 2008.08.12D23:12:24.018691392
    time.as_mut_slice::<J>().copy_from_slice(&[119067859167018272_i64, 201766609419710368, 271897944018691392]);
    let temperature = new_list(qtype::FLOAT_LIST, 3);
    temperature.as_mut_slice::<F>().copy_from_slice(&[22.1_f64, 24.7, 30.5]);
    values.as_mut_slice::<K>().copy_from_slice(&[time, temperature]);
    
    flip(new_dictionary(keys, values))
}
```

And q code is here:

```q
q)summon:`libc_api_examples 2: (`create_symbol_list2; 1)
q)summon[]
`Abraham`Isaac`Jacob`Joseph
q)chill: `libc_api_examples 2: (`no_panick; 2);
q)chill[$; ("J"; "42")]
success!
42
q)chill[+; (1; `a)]
FYI: type
q)climate_change: libc_api_examples 2: (`create_table2; 1);
q)climate_change[]
time                          temperature
-----------------------------------------
2003.10.10D02:24:19.167018272 22.1       
2006.05.24D06:16:49.419710368 24.7       
2008.08.12D23:12:24.018691392 30.5  
```

## Test

Testing is conducted in two ways:

1. Using cargo
2. Running a q test script

### 1. Using Cargo

Before starting the test, start a kdb+ process listening on port 5000.

```bash
$ q -p 5000
q)
```

Then run the test:

```bash
kxkdb]$ cargo test
```

**Note:** Currently 20 tests fails for `api` examples in document. This is because the examples do not have `main` function by nature of `api` but still use `#[macro_use]`.

### 2. Running a q Test Script

Tests are conducted with `tests/test.q` by loading the example functions built in `api_examples`.

```bash
kxkdb]$ cargo build --release
kxkdb]$ cp target/release/libapi_examples.so tests/
kxkdb]$ cd tests
tests]$ q test.q
Initialized something, probably it is your mindset.
bool: true
bool: false
byte: 0xc4
GUID: 8c6b-8b-64-68-156084
short: 10
int: 42
int: 122
int: 7336
int: 723
int: 14240
int: 2056636
long: -109210
long: 43200123456789
long: -325389000000021
long: 0
real: 193810.31
float: -37017.09330000
float: 742.41927468
char: "k"
symbol: `locust
string: "gnat"
string: "grasshopper"
error: type
What do you see, son of man?: a basket of summer fruit
What do you see, son of man?: boiling pot, facing away from the north
symbol: `rust
success!
FYI: type
this is KNULL
Planet { name: "earth", population: 7500000000, water: true }
Planet { name: "earth", population: 7500000000, water: true }
おいしい！
おいしい！
おいしい！
おいしい！
おいしい！
おいしい！
おいしい！
おいしい！
おいしい！
おいしい！
"Collect the clutter of apples!"
test result: ok. 147 passed; 0 failed
q)What are the three largest elements?: `belief`love`hope
```

# The Medal Contest Platform

![buildstatus](https://git.bwinf.de/zgtm/medal-prototype/badges/master/build.svg) [![forthebadge](https://forthebadge.com/images/badges/fuck-it-ship-it.svg)](https://forthebadge.com)

Medal is a small platform for in-browser running contest written in rust.

It is designed for the German Jugendwettbewerb Informatik, a computer science contest with tasks using Google Blockly as a programming language-replacement.




## Translation

…


## Folder structure

### tasks/


## Running Medal

Needs `rustc` 1.28 (stable) or higher (cf. https://rustup.rs/). 

Running 
```
make
```
compiles and runs a debug-/test-server.

For production use, a release binary should be compiled and served behind a reverse proxy (nginx, apache, …). 
```
make release
```
compiles a release build with openssl statically linked for distribution.

The directories `tasks/` and `static/` can (and for throughput-purposes should) be served by the reverse proxy directly.

## Deploy

It is recommended to run the platform behind a reverse proxy, that is serving static files directly.

The following configuration can be used for an Apache 2.4 webserver:

```
  ServerSignature Off
  ProxyPreserveHost On
  AllowEncodedSlashes NoDecode
  
  ProxyPass /static/ !
  ProxyPass /tasks/ !
  ProxyPass /favicon.ico !
  ProxyPass / http://[::1]:8080/
  ProxyPassReverse / http://[::1]:8080/
  
  Alias "/tasks/" "/path/to/medal/tasks/"
  Alias "/static/" "/path/to/medal/static/"
  Alias "/favicon.ico" "/path/to/medal/static/images/favicon.png"

  <filesMatch ".(css|jpg|jpeg|png|gif|js|ico)$">
    Header set Cache-Control "max-age=604800, public"
  </filesMatch>

  <Directory "/path/to/medal/static/">
    Require all granted
  </Directory>
 
   
  <Directory "/path/to/medal/tasks/">
    Require all granted
  </Directory>
```

## Contributing

Please format your code with `rustfmt` and check it for warnings with `clippy`.

You can install those with 
```
rustup component add rustfmt --toolchain nightly
rustup component add clippy
```

Format the code and check for warnings with
```
make format
make clippy
```


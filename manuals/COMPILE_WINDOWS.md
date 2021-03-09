# How to run Medal under Windows (64 bit)

Prerequisites: Rust, Cargo and git are already installed

## 1. Install vcpkg 
Description by Microsoft: https://docs.microsoft.com/en-us/cpp/build/vcpkg?view=vs-2019

Basically:
1. Clone git repository from https://github.com/Microsoft/vcpkg
2. In new folder „vcpkg“ run „bootstrap-vcpkg.bat“

## 2. Install OpenSSL and sqlite3 with vcpkg
1. Go to the folder vcpkg via the commandline
2. Enter the following commands:
```
vcpkg install openssl:x64-windows
vcpkg install sqlite:x64-windows
```

## 3. Set the environment variables:
The following environment variables need to be set:
   - OPENSSL_DIR should point to „vcpkg\packages\openssl-windows_x64-windows“ (This means in my case: `C:\Users\<Nutzername>\Documents\vcpkg\packages\openssl-windows_x64-windows`)
   - OPENSSL_LIB_DIR should point to „vcpkg\packages\openssl-windows_x64-windows\lib“
   - SQLITE3_LIB_DIR should point to „vcpkg\packages\sqlite3_x64-windows\lib“ 

## 4. Copy sqlite3.dll into the right place
Copy „sqlite3.dll“ from `\vcpkg\packages\sqlite3_x64-windows\bin` to `medal-prototype`.

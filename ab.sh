 # To install apache benchmark 
 # ubuntu - sudo apt install apache2-utils
 # arch linux - sudo pacman -Sy & sudo pacman -S apache-tools
RUST_PORT=8080
JS_PORT=3000
echo "Benchmarking RUST"
ab -n 10000 -c 1000 "http://localhost:$RUST_PORT/users/search?size=50" > rust_bench
echo "Benchmarking JS"
ab -n 10000 -c 1000 "http://localhost:$JS_PORT/" > js_bench
echo "Diff bench output"
diff rust_bench js_bench 
rm js_bench rust_bench
// This is a rhai script that will reverse the incoming
// bytes if the target is "target1"
fn reverse_array(arr) {
    let len = arr.len();
    let mid = len / 2;
    for i in 0..mid {
        let temp = arr[i];
        arr[i] = arr[len - 1 - i];
        arr[len - 1 - i] = temp;
    }
    arr
}

if target == "target1" {
    let reversed_data = reverse_array(data);
    return reversed_data
}

data

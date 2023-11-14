if target == "target2" {
    let file = open_file("/tmp/target2.log", "a");
    file.write("let data = [");
    for b in data {
        file.write(`${b}`);
        file.write(",");
    }
    file.write("];\n");
}

data

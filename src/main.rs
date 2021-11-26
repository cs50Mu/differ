use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::collections::HashSet;
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
// use std::env;
use clap::Parser;
use std::fs::OpenOptions;
use std::str::FromStr;

#[derive(Parser)]
#[clap(version = "0.1", author = "linuxfish <linuxfish.exe@gmail.com>")]
struct DifferConfig {
    /// 要比对的文件1
    path1: String,
    /// 要比对的文件2
    path2: String,
    /// 基于哪个列来比对, count from 1, eg: 1:1
    check_field: PairFields,
    /// 要选择输出的列, 可选, 默认输出全部列, eg: 1,2:3,6,9
    output_fields: Option<PairFields>,
}

#[derive(Debug, PartialEq)]
struct PairFields(Vec<usize>, Vec<usize>);

impl FromStr for PairFields {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // 1,2:3,4
        let pair_config = s.split(':').collect::<Vec<_>>();
        if pair_config.len() != 2 {
            return Err(anyhow!(format!("bad format: {}", s)));
        }
        println!("pair_config: {:?}", pair_config);

        

        // // 合在一起的写法, 下面分开写的方式似乎更清楚一点,不过效果是完全一样的
        // let xx = pair_config
        //     .iter()
        //     .map(|fields| {
        //         if fields.trim().is_empty() {
        //             return Err(anyhow!("bad format: {}", s));
        //         }
        //         let splitted = fields.split(',').collect::<Vec<_>>();
        //         println!("{:?}", splitted);
        //         if splitted.is_empty() {
        //             return Err(anyhow!(format!("bad format: {}", s)));
        //         }
        //         splitted
        //             .iter()
        //             .map(|s| Ok(s.parse::<usize>()?))
        //             .collect::<Result<Vec<_>>>()
        //     })
        //     .collect::<Result<Vec<Vec<usize>>>>()?;

        // Ok(Self(xx[0].clone(), xx[1].clone()))


        // collect 成一个 Result<Vec<_>> 一个处理 fallible iteration 的方法
        let yy: Result<Vec<Vec<_>>> = pair_config
            .iter()
            .map(|fields| {
                if fields.trim().is_empty() {
                    return Err(anyhow!("bad format: {}", s));
                }
                let splitted = fields.split(',').collect::<Vec<_>>();
                // println!("{:?}", splitted);
                if splitted.is_empty() {
                    return Err(anyhow!(format!("bad format: {}", s)));
                }
                Ok(splitted)
            }).collect();

        // 注意这里, 可以直接 collect 成一个<Result<Vec<usize>>>, 并且在
        // 遇到错误的时候会停止处理
        // 因为是返回一个二维数组, 所以需要两次 collect
        // 参考: https://stackoverflow.com/questions/26368288/how-do-i-stop-iteration-and-return-an-error-when-iteratormap-returns-a-result
        // https://morestina.net/blog/1607/fallible-iteration
        let zz = yy?
            .iter()
            .map(|vv | {
                vv
                .iter()
                // 这里需要手动把 error 映射成 anyhow::Error,为啥不能自动转呢?
                // .map(|field| field.parse::<usize>().map_err(|e| anyhow!(e)))
                // 外面加包一个Ok就好了...
                .map(|field| Ok(field.parse::<usize>()?))
                // 下面这种写法会报错是因为 "the `?` operator can only be used in a closure that returns `Result` or `Option`"
                // 所以包个 Ok 就好了 
                // .map(|field| field.parse::<usize>()?)
                .collect::<Result<Vec<usize>>>()
            }).collect::<Result<Vec<Vec<usize>>>>()?;

        Ok(Self(zz[0].clone(), zz[1].clone()))
    }
}

/// 输入的两个文件, 其内容需要是 uniq 的, 要比较的
/// 列不能有重复的值
/// TODO: 可以在比较之前先预处理一下, 剔除重复的
fn main() -> Result<()> {
    let config: DifferConfig = DifferConfig::parse();

    println!(
        "check_field: {:?}\n output_fields: {:?}",
        config.check_field, config.output_fields
    );

    // let args: Vec<String> = env::args().collect();

    // args[0] 是当前的可执行文件!
    // let m1 = read_file(&args[1], 0)?;
    // let m2 = read_file(&args[2], 0)?;
    let m1 = read_file(&config.path1, config.check_field.0[0] - 1)?;
    let m2 = read_file(&config.path2, config.check_field.1[0] - 1)?;

    // simple way to gen a hashset from a hashmap
    let s1: HashSet<_> = m1.keys().cloned().collect();
    let s2: HashSet<_> = m2.keys().cloned().collect();

    // println!("
    //     s1 - s2: {:?}\n
    //     s2 - s1: {:?}\n
    //     intersect of s1 and s2: {:?}",
    //     s1.difference(&s2),
    //     s2.difference(&s1),
    //     s1.intersection(&s2));

    handle_difference(&s1, &s2, &m1, "a-b.csv")?;

    handle_difference(&s2, &s1, &m2, "b-a.csv")?;

    if let Some(output_fields) = config.output_fields {
        handle_intersection(&s1, &s2, &m1, &m2, &output_fields.0, &output_fields.1)?;
    } else {
        handle_intersection(&s1, &s2, &m1, &m2, &[], &[])?;
    }

    Ok(())
}

fn read_file(path: &str, idx: usize) -> Result<HashMap<String, Vec<String>>> {
    let f = File::open(path)?;
    let map = BufReader::new(f)
        .lines()
        .filter_map(|line| line.ok())
        .map(|line| {
            let line = line.trim();
            let splitted = line.split(',').map(|s| s.to_owned()).collect::<Vec<_>>();
            (splitted[idx].to_string(), splitted)
        })
        .collect();
    Ok(map)
}

fn handle_difference(
    s1: &HashSet<String>,
    s2: &HashSet<String>,
    m: &HashMap<String, Vec<String>>,
    file_name: &str,
) -> Result<()> {
    let f = OpenOptions::new()
        .append(true)
        .create(true)
        .open(file_name)?;

    let mut f = BufWriter::new(f);
    // hashset 可以直接 collect 成 Vec<_>
    s1.difference(s2).collect::<Vec<_>>().
        iter().
        // for_each(| item | f.write_all(item.as_bytes()).unwrap());
        for_each(| &item | {
            let fields = &m[item];
            // https://stackoverflow.com/questions/32472495/how-do-i-write-a-formatted-string-to-a-file
            // https://www.philipdaniels.com/blog/2019/rust-file-io/
            writeln!(f, "{}", fields.join(",")).unwrap()
        });
    Ok(())
}

fn handle_intersection(
    s1: &HashSet<String>,
    s2: &HashSet<String>,
    m1: &HashMap<String, Vec<String>>,
    m2: &HashMap<String, Vec<String>>,
    fields1: &[usize],
    fields2: &[usize],
) -> Result<()> {
    let f = OpenOptions::new()
        .append(true)
        .create(true)
        .open("intersect.csv")?;

    let mut f = BufWriter::new(f);
    s1.intersection(s2)
        .collect::<Vec<_>>()
        .iter()
        .for_each(|&item| {
            // difference between `iter` and `into_iter`?
            // https://stackoverflow.com/a/57697614/1543462
            // https://hermanradtke.com/2015/06/22/effectively-using-iterators-in-rust.html
            let mut f1 = fields1
                .iter()
                .map(|&idx| &m1[item][idx - 1])
                .collect::<Vec<_>>();
            let f2 = fields2
                .iter()
                .map(|&idx| &m2[item][idx - 1])
                .collect::<Vec<_>>();
            f1.extend(f2);
            // TODO: 不 clone 的话下面一行就会报 'borrow of moved value'
            let mut out = f1.clone();
            if f1.is_empty() {
                let mut fields1 = m1[item].iter().collect::<Vec<_>>();
                let fields2 = m2[item].iter().collect::<Vec<_>>();
                fields1.extend(fields2);
                // copied 的用处是把一个 iterator over &T 变成一个 iterator over T
                let tmp = fields1.iter().copied().collect::<Vec<_>>();
                out = tmp;
            }
            // 如何写文件
            // https://stackoverflow.com/questions/32472495/how-do-i-write-a-formatted-string-to-a-file
            // https://www.philipdaniels.com/blog/2019/rust-file-io/
            writeln!(
                f,
                "{}",
                out.iter()
                    .map(|s| s.to_string())
                    .collect::<Vec<_>>()
                    .join(",")
            )
            .unwrap()
        });
    Ok(())
}
# WildFly Container Versions

A library for managing WildFly container versions deployed at https://hub.docker.com/r/jboss/wildfly and
https://quay.io/repository/wildfly/wildfly.

The library contains a struct describing WildFly container versions and various functions to parse expressions that are either
short versions, multipliers, ranges, enumerations, or a combination of them.

The library is consumed by Rust-based CLI tools that deal with WildFly containers such as [wado](https://github.com/hpehl/wado).

```rust
use semver::Version;

#[derive(Debug, Eq, PartialEq, Hash, Clone)]
pub struct WildFlyContainer {
    pub identifier: u16,
    pub version: Version,
    pub short_version: String,
    pub core_version: Version,
    pub suffix: String,
    pub repository: String,
    pub platforms: Vec<String>,
}
```

```rust
use anyhow::Result;
use wildfly_container_versions::WildFlyContainer;

fn main() -> Result<()> {
    let enumeration: Vec<WildFlyContainer> = WildFlyContainer::enumeration("3x10,23..26,5x28,34")?;
    let range: Vec<WildFlyContainer> = WildFlyContainer::range("26.1..29")?;
    let versions: Vec<WildFlyContainer> = WildFlyContainer::versions("2x33")?;
    let version: WildFlyContainer = WildFlyContainer::version("35")?;
    let lookup: WildFlyContainer = WildFlyContainer::lookup(340)?;
    Ok(())
}
```

## Version Expressions

Version expressions are either short versions, multipliers, ranges, enumerations, or a combination of them. They follow
this [BNF](https://bnfplayground.pauliankline.com/?bnf=%3Cexpression%3E%20%3A%3A%3D%20%3Cexpression%3E%20%22%2C%22%20%3Celement%3E%20%7C%20%3Celement%3E%0A%3Celement%3E%20%3A%3A%3D%20%3Cmultiplier%3E%20%22x%22%20%3Crange%3E%20%7C%20%3Cmultiplier%3E%20%22x%22%20%3Cshort_version%3E%20%7C%20%3Crange%3E%20%7C%20%3Cshort_version%3E%0A%3Crange%3E%20%3A%3A%3D%20%3Cshort_version%3E%20%22..%22%20%3Cshort_version%3E%20%7C%20%22..%22%20%3Cshort_version%3E%20%7C%20%3Cshort_version%3E%20%22..%22%20%7C%20%22..%22%0A%3Cmultiplier%3E%20%3A%3A%3D%20%3Cnonzero_number%3E%20%7C%20%3Ctwo_digit_number%3E%0A%3Cshort_version%3E%20%3A%3A%3D%20%3Cmajor%3E%20%7C%20%3Cmajor%3E%20%22.%22%20%3Cminor%3E%0A%3Cmajor%3E%20%3A%3A%3D%20%3Ctwo_digit_number%3E%20%7C%20%3Cthree_digit_number%3E%0A%3Cminor%3E%20%3A%3A%3D%20%3Cnonzero_number%3E%20%7C%20%3Ctwo_digit_number%3E%0A%3Cthree_digit_number%3E%20%3A%3A%3D%20%3Cnonzero_number%3E%20%3Cnumber%3E%20%3Cnumber%3E%0A%3Ctwo_digit_number%3E%20%3A%3A%3D%20%3Cnonzero_number%3E%20%3Cnumber%3E%0A%3Cnumber%3E%20%3A%3A%3D%20%220%22%20%7C%20%221%22%20%7C%20%222%22%20%7C%20%223%22%20%7C%20%224%22%20%7C%20%225%22%20%7C%20%226%22%20%7C%20%227%22%20%7C%20%228%22%20%7C%20%229%22%0A%3Cnonzero_number%3E%20%3A%3A%3D%20%221%22%20%7C%20%222%22%20%7C%20%223%22%20%7C%20%224%22%20%7C%20%225%22%20%7C%20%226%22%20%7C%20%227%22%20%7C%20%228%22%20%7C%20%229%22%0A&name=WildFly%20Container%20Versions):

```
<expression> ::= <expression> "," <element> | <element>
<element> ::= <multiplier> "x" <range> | <multiplier> "x" <short_version> | <range> | <short_version>
<range> ::= <short_version> ".." <short_version> | ".." <short_version> | <short_version> ".." | ".."
<multiplier> ::= <nonzero_number> | <two_digit_number>
<short_version> ::= <major> | <major> "." <minor>
<major> ::= <two_digit_number> | <three_digit_number>
<minor> ::= <nonzero_number> | <two_digit_number>
<three_digit_number> ::= <nonzero_number> <number> <number>
<two_digit_number> ::= <nonzero_number> <number>
<number> ::= "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9"
<nonzero_number> ::= "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9"
```

### Examples

- 10
- 26.1
- 3x35
- 23..33
- 25..
- ..26.1
- ..
- 5x33..35
- 20,25..29,2x31,3x32,4x33..35

## Supported Versions

| Short Version | WildFly Version | Image / Tag                                  | Platforms                                            |
|---------------|-----------------|----------------------------------------------|------------------------------------------------------|
| 10            | 10.0.0.Final    | docker.io/jboss/wildfly:10.0.0.Final         |                                                      |
| 10.1          | 10.1.0.Final    | docker.io/jboss/wildfly:10.1.0.Final         |                                                      |
| 11            | 11.0.0.Final    | docker.io/jboss/wildfly:11.0.0.Final         |                                                      |
| 12            | 12.0.0.Final    | docker.io/jboss/wildfly:12.0.0.Final         |                                                      |
| 13            | 13.0.0.Final    | docker.io/jboss/wildfly:13.0.0.Final         |                                                      |
| 14            | 14.0.1.Final    | docker.io/jboss/wildfly:14.0.1.Final         |                                                      |
| 15            | 15.0.1.Final    | docker.io/jboss/wildfly:15.0.1.Final         |                                                      |
| 16            | 16.0.0.Final    | docker.io/jboss/wildfly:16.0.0.Final         |                                                      |
| 17            | 17.0.1.Final    | docker.io/jboss/wildfly:17.0.1.Final         |                                                      |
| 18            | 18.0.1.Final    | docker.io/jboss/wildfly:18.0.1.Final         |                                                      |
| 19            | 19.0.0.Final    | docker.io/jboss/wildfly:19.0.0.Final         |                                                      |
| 19.1          | 19.1.0.Final    | docker.io/jboss/wildfly:19.1.0.Final         |                                                      |
| 20            | 20.0.1.Final    | docker.io/jboss/wildfly:20.0.1.Final         |                                                      |
| 21            | 21.0.2.Final    | docker.io/jboss/wildfly:21.0.2.Final         |                                                      |
| 22            | 22.0.1.Final    | docker.io/jboss/wildfly:22.0.1.Final         |                                                      |
| 23            | 23.0.2.Final    | quay.io/wildfly/wildfly:23.0.2.Final         |                                                      |
| 24            | 24.0.1.Final    | quay.io/wildfly/wildfly:24.0.1.Final         |                                                      |
| 25            | 25.0.1.Final    | quay.io/wildfly/wildfly:25.0.1.Final         |                                                      |
| 26            | 26.0.1.Final    | quay.io/wildfly/wildfly:26.0.1.Final         |                                                      |
| 26.1          | 26.1.3.Final    | quay.io/wildfly/wildfly:26.1.3.Final-jdk17   | linux/amd64, linux/arm64                             |
| 27            | 27.0.1.Final    | quay.io/wildfly/wildfly:27.0.1.Final-jdk19   | linux/amd64, linux/arm64                             |
| 28            | 28.0.1.Final    | quay.io/wildfly/wildfly:28.0.1.Final-jdk20   | linux/amd64, linux/arm64                             |
| 29            | 29.0.1.Final    | quay.io/wildfly/wildfly:29.0.1.Final-jdk20   | linux/amd64, linux/arm64                             |
| 30            | 30.0.1.Final    | quay.io/wildfly/wildfly:30.0.1.Final-jdk20   | linux/amd64, linux/arm64                             |
| 31            | 31.0.1.Final    | quay.io/wildfly/wildfly:31.0.1.Final-jdk20   | linux/amd64, linux/arm64                             |
| 32            | 32.0.1.Final    | quay.io/wildfly/wildfly:32.0.1.Final-jdk21   | linux/amd64, linux/arm64, linux/s390x                |
| 33            | 33.0.2.Final    | quay.io/wildfly/wildfly:33.0.2.Final-jdk21   | linux/amd64, linux/arm64, linux/s390x, linux/ppc64le |
| 34            | 34.0.1.Final    | quay.io/wildfly/wildfly:34.0.1.Final-jdk21   | linux/amd64, linux/arm64, linux/s390x, linux/ppc64le |
| 35            | 35.0.1.Final    | quay.io/wildfly/wildfly:35.0.1.Final-jdk21   | linux/amd64, linux/arm64, linux/s390x, linux/ppc64le |
| 36            | 36.0.1.Final    | quay.io/wildfly/wildfly:36.0.1.Final-jdk21   | linux/amd64, linux/arm64, linux/s390x, linux/ppc64le |
| 37            | 37.0.1.Final    | quay.io/wildfly/wildfly:37.0.1.Final-jdk21   | linux/amd64, linux/arm64, linux/s390x, linux/ppc64le |
| 38            | 38.0.1.Final    | quay.io/wildfly/wildfly:38.0.1.Final-jdk21   | linux/amd64, linux/arm64, linux/s390x, linux/ppc64le |
| 39            | 39.0.1.Final    | quay.io/wildfly/wildfly:39.0.1.Final-2-jdk21 | linux/amd64, linux/arm64, linux/s390x, linux/ppc64le |

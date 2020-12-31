use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};

use anyhow::*;

use graphite::*;

pub type Face = A3<I>;

#[derive(Default)]
pub struct MeshData {
    pub p:  Vec<P>,
    pub n:  Vec<N>,
    pub uv: Vec<F2>,
}

pub fn load_from_file(file: &str, to_world: T) -> Result<(MeshData, Vec<Face>)>
{
    let f = File::open(file)
                 .with_context(|| format!("Error opening OBJ file: {}", file))?;
    ObjLoader::new(to_world).load(BufReader::new(f))
}

#[derive(Default)]
struct ObjLoader {
    tmp_data:   MeshData,
    obj_data:   MeshData,
    faces:      Vec<Face>,
    vertex_map: HashMap<Vertex, I>,
    to_world:   T,
}

#[derive(Eq, Hash, PartialEq)]
struct Vertex {
    p: I,
    t: I,
    n: I,
}

impl ObjLoader {
    fn new(to_world: T) -> ObjLoader {
        ObjLoader { to_world, ..Default::default() }
    }

    fn load(mut self, mut buf: impl BufRead) -> Result<(MeshData, Vec<Face>)> {
        let mut line = String::with_capacity(120);
        while buf.read_line(&mut line).context("Error reading line")?  > 0 {
            let mut tokens = line[..].split_whitespace();

            match tokens.next() {
                Some("v") => self.add_point(&mut tokens),
                Some("vt") => self.add_uv(&mut tokens),
                Some("vn") => self.add_normal(&mut tokens),
                Some("f") => self.add_face(&mut tokens),
                _ => Ok(()),
            }?;

            line.clear();
        }

        Ok((self.obj_data, self.faces))
    }

    fn add_point<'a>(&mut self, tokens: &mut impl Iterator<Item = &'a str>)
        -> Result<()>
    { Ok(self.tmp_data.p.push(self.to_world * P::from(parse_f3(tokens)?))) }

    fn add_uv<'a>(&mut self, tokens: &mut impl Iterator<Item = &'a str>)
        -> Result<()>
    { Ok(self.tmp_data.uv.push(parse_f2(tokens)?)) }

    fn add_normal<'a>(&mut self, tokens: &mut impl Iterator<Item = &'a str>)
        -> Result<()>
    { Ok(self.tmp_data.n.push(self.to_world * N::from(parse_f3(tokens)?))) }

    fn add_face<'a>(&mut self, tokens: &mut impl Iterator<Item = &'a str>)
        -> Result<()>
    {
        let vertices: Result<Vec<I>, _> =
            tokens.map(|st| match self.parse_vertex(st) {
                      Ok(v) => match self.vertex_map.get(&v) {
                          Some(&i) => Ok(i),
                          None => Ok(self.add_vertex(v)),
                      },
                      Err(e) => Err(e),
                  })
                  .collect();
        let v = vertices?;

        match v.len() {
            3 => self.faces.push(A3(v[0], v[1], v[2])),
            4 => {
                self.faces.push(A3(v[0], v[1], v[2]));
                self.faces.push(A3(v[0], v[2], v[3]));
            }
            _ => bail!("unexpected number of vertices"),
        }
        Ok(())
    }

    fn add_vertex(&mut self, v: Vertex) -> I {
        self.obj_data.p.push(self.tmp_data.p[v.p as usize]);
        if v.t != -1 {
            self.obj_data.uv.push(self.tmp_data.uv[v.t as usize]);
        }
        if v.n != -1 {
            self.obj_data.n.push(self.tmp_data.n[v.n as usize]);
        }
        let n = self.vertex_map.len() as I;
        self.vertex_map.insert(v, n);
        n
    }

    fn parse_vertex(&mut self, token: &str) -> Result<Vertex> {
        let mut tokens = token.split('/');
        Ok(Vertex {
            p: parse_index(&mut tokens, self.tmp_data.p.len())
                .context("index for position is required")?,
            t: parse_index(&mut tokens, self.tmp_data.uv.len()).unwrap_or(-1),
            n: parse_index(&mut tokens, self.tmp_data.n.len()).unwrap_or(-1),
        })
    }
}

fn parse_index<'a>(tkns: &mut impl Iterator<Item = &'a str>, n: usize)
    -> Result<I>
{ parse(tkns).map(|i: I| if i > 0 { i - 1 } else { i + n as I }) }

fn parse_f3<'a>(tokens: &mut impl Iterator<Item = &'a str>) -> Result<F3>
{ Ok(A3(parse(tokens)?, parse(tokens)?, parse(tokens)?)) }

fn parse_f2<'a>(tokens: &mut impl Iterator<Item = &'a str>) -> Result<F2>
{ Ok(A2(parse(tokens)?, parse(tokens)?)) }

fn parse<'a, S>(tokens: &mut impl Iterator<Item = &'a str>) -> Result<S>
    where S: std::str::FromStr,
          <S as std::str::FromStr>::Err: std::error::Error + Sync + Send
                                       + 'static
{ Ok(tokens.next().context("missing scalar")?.parse::<S>()?) }

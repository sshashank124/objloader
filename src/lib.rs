use std::collections::HashMap;
use std::io::{BufRead, BufReader};

use graphite::*;

pub type Face = A3<I>;

type Res<T> = Result<T, String>;

#[derive(Default)]
pub struct MeshData {
    pub p:  Vec<P>,
    pub n:  Vec<N>,
    pub uv: Vec<F2>,
}

pub fn load_from_file(file: &str, to_world: T) -> Res<(MeshData, Vec<Face>)> {
    let f = std::fs::File::open(file).map_err(|e| {
                                         format!("Error opening OBJ file: {}",
                                                 e)
                                     })?;
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

    fn load(mut self, mut buf: impl BufRead) -> Res<(MeshData, Vec<Face>)> {
        let mut line = String::with_capacity(120);
        while buf.read_line(&mut line)
                 .map_err(|e| format!("Error reading line: {}", e))?
              > 0
        {
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

    fn add_point<'a>(&mut self,
                     tokens: &mut impl Iterator<Item = &'a str>)
                     -> Res<()> {
        self.tmp_data.p.push(self.to_world * P::from(parse_f3(tokens)?));
        Ok(())
    }

    fn add_uv<'a>(&mut self,
                  tokens: &mut impl Iterator<Item = &'a str>)
                  -> Res<()> {
        self.tmp_data.uv.push(parse_f2(tokens)?);
        Ok(())
    }

    fn add_normal<'a>(&mut self,
                      tokens: &mut impl Iterator<Item = &'a str>)
                      -> Res<()> {
        self.tmp_data.n.push(self.to_world * N::from(parse_f3(tokens)?));
        Ok(())
    }

    fn add_face<'a>(&mut self,
                    tokens: &mut impl Iterator<Item = &'a str>)
                    -> Res<()> {
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
            3 => self.faces.push([v[0], v[1], v[2]].into()),
            4 => {
                self.faces.push([v[0], v[1], v[2]].into());
                self.faces.push([v[0], v[2], v[3]].into());
            }
            _ => return Err("unexpected number of vertices".into()),
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

    fn parse_vertex(&mut self, token: &str) -> Res<Vertex> {
        let mut tokens = token.split('/');
        Ok(Vertex {
            p: parse_index(&mut tokens, self.tmp_data.p.len())
                .map_err(|e| format!("index for position is required: {}", e))?,
            t: parse_index(&mut tokens, self.tmp_data.uv.len()).unwrap_or(-1),
            n: parse_index(&mut tokens, self.tmp_data.n.len()).unwrap_or(-1),
        })
    }
}

fn parse_index<'a>(tkns: &mut impl Iterator<Item = &'a str>,
                   n: usize)
                   -> Res<I> {
    parse(tkns).map(|i: I| if i > 0 { i - 1 } else { i + n as I })
}

fn parse_f3<'a>(tokens: &mut impl Iterator<Item = &'a str>) -> Res<F3> {
    Ok([parse(tokens)?, parse(tokens)?, parse(tokens)?].into())
}

fn parse_f2<'a>(tokens: &mut impl Iterator<Item = &'a str>) -> Res<F2> {
    Ok([parse(tokens)?, parse(tokens)?].into())
}

fn parse<'a, S>(tokens: &mut impl Iterator<Item = &'a str>) -> Res<S>
    where S: std::str::FromStr,
          <S as std::str::FromStr>::Err: std::fmt::Display
{
    tokens.next()
          .ok_or("missing scalar")?
          .parse::<S>()
          .map_err(|e| format!("malformed scalar: {}", e))
}

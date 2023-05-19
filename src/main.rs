#![feature(box_syntax, box_patterns)]
pub mod lambda;
use crate::lambda::{Expr, PLUS, POWER, SUCC, TIMES};
use std::io::stdin;
use Expr::*;

fn main() -> Result<(), std::io::Error> {
  let mut expr = Expr::Var(255);
  let mut cursor = Expr::Hole;
  let mut keys = stdin().lines().map(|l| l.unwrap()).flat_map(|l| {
    l.split_whitespace()
      .map(|w| w.to_owned())
      .collect::<Vec<_>>()
  });
  println!("{}", expr);
  println!("[255] = {}", cursor);
  while let Some(cmd) = keys.next() {
    match cmd.as_str() {
      "bs" => cursor = Expr::Hole,
      "l" => cursor = Expr::Lam(box cursor),
      "b" => {
        if !cursor.beta() {
          println!("boop(beta)")
        }
      }
      "dn" => match cursor {
        Lam(box e) => {
          expr.replace(255, &Lam(box Var(255)));
          cursor = e
        }
        App(box l, r) => {
          expr.replace(255, &App(box Var(255), r));
          cursor = l
        }
        Hole | Var(_) => println!("boop"),
      },
      "up" => {
        if let App(box Hole, box e) | App(box e, box Hole) = cursor {
          cursor = e;
        }
        if !expr.map_parent(255, &mut |e| {
          let mut new = Var(255);
          std::mem::swap(e, &mut new);
          new.replace(255, &cursor);
          cursor = new;
        }) {
          println!("boop")
        }
      }
      "top" => {
        expr.replace(255, &cursor);
        cursor = expr;
        expr = Var(255)
      }
      "lt" => {
        if !expr.map_parent(255, &mut |e| match e {
          App(box l, box r) if *r == Var(255) => {
            std::mem::swap(r, &mut cursor);
            std::mem::swap(l, &mut cursor);
          }
          _ => todo!(),
        }) {
          println!("boop")
        }
      }
      "+" => match cursor {
        Hole => cursor = PLUS.clone(),
        _ => cursor = App(box PLUS.clone(), box cursor),
      },
      "*" => match cursor {
        Hole => cursor = TIMES.clone(),
        _ => cursor = App(box TIMES.clone(), box cursor),
      },
      "^" => match cursor {
        Hole => cursor = POWER.clone(),
        _ => cursor = App(box POWER.clone(), box cursor),
      },
      s => {
        if let Ok(u) = s.parse::<u8>() {
          if cursor == Hole {
            cursor = Expr::from_nat(u)
          } else {
            println!("boop")
          }
        } else {
          println!("unrec'd cmd: {}", s)
        }
      }
    }
    println!("{}", expr);
    println!("[255] = {}", cursor);
  }
  Ok(())
}

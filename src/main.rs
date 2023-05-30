#![feature(box_syntax, box_patterns)]
pub mod lambda;
use crate::lambda::{DisplayStruct, Expr, PLUS, POWER, TIMES};
use std::{io::stdin, mem};
use Expr::*;

fn main() {
  let mut expr = Slot;
  let mut cursor = Hole;
  let keys = stdin()
    .lines()
    .map(std::result::Result::unwrap)
    .flat_map(|l| {
      l.split_whitespace()
        .map(std::borrow::ToOwned::to_owned)
        .collect::<Vec<_>>()
    });
  let mut leaf_mode = true;
  for cmd in keys {
    match cmd.as_str() {
      "bs" => cursor = Hole,
      "l" => {
        if cursor == Hole {
          expr.replace_slot(Lam(box Slot));
        } else {
          cursor = Lam(box cursor);
        }
      }
      "b" => {
        if !cursor.beta() {
          println!("boop(beta)");
        }
      }
      "redux" => {
        if let Some(hd) = cursor.find_redux() {
          let mut new = Slot;
          mem::swap(hd, &mut new);
          expr.replace_slot(cursor);
          cursor = new;
        } else {
          println!("boop(redux)");
        }
      }
      "dn" => match cursor {
        Lam(box e) => {
          expr.replace_slot(Lam(box Slot));
          cursor = e;
        }
        App(box l, r) => {
          expr.replace_slot(App(box Slot, r));
          cursor = l;
        }
        Hole | Var(_) => {
          if leaf_mode {
            println!("boop");
          } else {
            leaf_mode = true;
          }
        }
        Slot => panic!(),
      },
      "up" => {
        if leaf_mode {
          leaf_mode = false;
        } else if let Some(p) = expr.find_slot_parent() {
          if let App(box Slot, box e) | App(box e, box Slot) = p && cursor == Hole {
            mem::swap(e, &mut cursor);
            *p = Slot;
          } else {
            let mut new = Slot;
            mem::swap(p, &mut new);
            new.replace_slot(cursor);
            cursor = new;
          }
        } else {
          println!("boop");
        }
      }
      "top" => {
        expr.replace_slot(cursor);
        cursor = expr;
        expr = Slot;
      }
      "lm" => {
        let mut new = Slot;
        mem::swap(cursor.leftmost(), &mut new);
        expr.replace_slot(cursor);
        cursor = new;
        leaf_mode = true;
      }
      "rm" => {
        let mut new = Slot;
        mem::swap(cursor.rightmost(), &mut new);
        expr.replace_slot(cursor);
        cursor = new;
        leaf_mode = true;
      }
      "lt" => {
        if leaf_mode {
          if let Some((slot, sib)) = expr.find_slot_leftsib() {
            mem::swap(slot, &mut cursor);
            mem::swap(sib, &mut cursor);
          } else {
            leaf_mode = false;
          }
        } else if let Some(p) = expr.find_slot_parent() {
          match p {
            App(box e, box Slot) if cursor == Hole => {
              mem::swap(e, &mut cursor);
              *p = Slot;
            }
            App(box l, box r) if *r == Slot => {
              mem::swap(r, &mut cursor);
              mem::swap(l, &mut cursor);
            }
            _ => {
              let mut new = Slot;
              mem::swap(p, &mut new);
              new.replace_slot(cursor);
              cursor = new;
            }
          }
        } else {
          println!("boop");
        }
      }
      "rt" => {
        if leaf_mode {
          if let Some((slot, sib)) = expr.find_slot_rightsib() {
            mem::swap(slot, &mut cursor);
            mem::swap(sib, &mut cursor);
          } else {
            leaf_mode = false;
          }
        } else if let Some(p) = expr.find_slot_parent() {
          match p {
            App(box Slot, box e) if cursor == Hole => {
              mem::swap(e, &mut cursor);
              *p = Slot;
            }
            App(box l, box r) if *l == Slot => {
              mem::swap(l, &mut cursor);
              mem::swap(r, &mut cursor);
            }
            _ => {
              let mut new = Slot;
              mem::swap(p, &mut new);
              new.replace_slot(cursor);
              cursor = new;
            }
          }
        } else {
          println!("boop");
        }
      }
      "$" => {
        expr.replace_slot(App(box cursor, box Slot));
        cursor = Hole;
      }
      "@" => {
        expr.replace_slot(App(box Slot, box cursor));
        cursor = Hole;
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
            cursor = Expr::from_nat(u);
          } else {
            cursor = App(box cursor, box Expr::from_nat(u));
          }
        } else if let Some(u) = s
          .strip_prefix('[')
          .and_then(|s| s.strip_suffix(']'))
          .and_then(|s| s.parse::<u8>().ok())
        {
          if cursor == Hole {
            cursor = Var(u);
          } else {
            cursor = App(box cursor, box Var(u));
          }
        } else {
          println!("unrec'd cmd: {}", s);
        }
      }
    }
    if !matches!(cursor, Var(_) | Hole) {
      leaf_mode = false;
    }
    let dstr = DisplayStruct {
      expr : &expr,
      cursor : &cursor,
      leaf_mode,
    };
    eprintln!("{:?}", dstr);
    println!("{}", dstr);
  }
}

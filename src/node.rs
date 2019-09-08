use regex::Regex;
use osmpbfreader::OsmObj;

#[derive(Debug)]
enum Operator {
    Equals,
    Includes,
}

#[derive(Debug)]
pub struct NodeExpression {
    tag_name: String,
    tag_value: String,
    op: Operator,
}

impl NodeExpression {
    pub fn parse(expression: String) -> Result<NodeExpression, String> {
        let re =
            Regex::new(r"(?P<name>\w+)(?P<op>[=~])(?P<value>\w+)").map_err(|e| e.to_string())?;
        let err = format!("Could not compile expression from {}", expression);
        let caps = re.captures(&expression).ok_or(err)?;

        let op = if caps.name("op").unwrap().as_str() == "=" {
            Operator::Equals
        } else {
            Operator::Includes
        };

        Ok(NodeExpression {
            tag_name: caps.name("name").unwrap().as_str().into(),
            tag_value: caps.name("value").unwrap().as_str().into(),
            op: op,
        })
    }

    pub fn matcher(&self) -> impl Fn(&OsmObj) -> bool + '_ {
        move |obj: &OsmObj| match self.op {
            Operator::Equals => obj.tags().contains(&self.tag_name, &self.tag_value),
            Operator::Includes => obj.tags().get(&self.tag_name).map_or(false, |v| {
                v.to_lowercase().contains(&self.tag_value.to_lowercase())
            }),
        }
    }
}

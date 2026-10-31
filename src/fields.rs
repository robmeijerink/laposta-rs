//! Fields of a list.

use reqwest::Method;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::client::{Body, Client, Query, decode, decode_list, to_value, with_field};
use crate::error::Result;
use crate::serde_util::string_or_number;
use crate::types::{FieldId, ListId};

/// Data type of a field.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum DataType {
    /// Free text.
    Text,
    /// A number.
    Numeric,
    /// A date.
    Date,
    /// One choice out of a set of options.
    SelectSingle,
    /// Multiple choices out of a set of options.
    SelectMultiple,
    /// The labels field, managed by Laposta. It cannot be created, changed or deleted.
    Labels,
    /// A data type this crate does not know about.
    #[default]
    #[serde(other)]
    Unknown,
}

/// Data type of a field that can be written through the API.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum WritableDataType {
    /// Free text.
    Text,
    /// A number.
    Numeric,
    /// A date.
    Date,
    /// One choice out of a set of options.
    SelectSingle,
    /// Multiple choices out of a set of options.
    SelectMultiple,
}

/// How a single-choice field is displayed in forms.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum DisplayType {
    /// A dropdown.
    Select,
    /// Radio buttons.
    Radio,
    /// A display type this crate does not know about. It cannot be sent.
    #[default]
    #[serde(other, skip_serializing)]
    Unknown,
}

/// Whether a field is in use or has been deleted.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum FieldState {
    /// The field is active.
    Active,
    /// The field was deleted.
    Deleted,
    /// A state this crate does not know about.
    #[default]
    #[serde(other)]
    Unknown,
}

/// A selectable option of a select field.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
#[non_exhaustive]
pub struct FieldOption {
    /// Option id. When updating a field, a numeric id refers to an existing
    /// option and any other value (for example `new-0`) adds a new one.
    #[serde(deserialize_with = "string_or_number")]
    pub id: String,
    /// Label of the option.
    pub value: String,
}

impl FieldOption {
    /// Creates an option from an id and a label.
    pub fn new(id: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            value: value.into(),
        }
    }
}

/// A field of a list.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
#[non_exhaustive]
pub struct Field {
    /// Field id.
    pub field_id: FieldId,
    /// The list the field belongs to.
    pub list_id: ListId,
    /// Account timezone used for the datetime fields.
    pub timezone: String,
    /// Creation time.
    pub created: String,
    /// Creation time in ISO 8601 format.
    pub created_iso: String,
    /// Always `None`; kept by the API for backwards compatibility.
    pub modified: Option<String>,
    /// Current state.
    pub state: FieldState,
    /// Display name.
    pub name: String,
    /// Variable to use in campaigns, for example `{{color}}`.
    pub tag: String,
    /// Name to use in `custom_fields` of member requests.
    pub custom_name: String,
    /// Value used when none is supplied.
    pub defaultvalue: Option<String>,
    /// Data type.
    pub datatype: DataType,
    /// Display type of a single-choice field.
    pub datatype_display: Option<DisplayType>,
    /// Option labels of a select or labels field.
    pub options: Vec<String>,
    /// Options including their ids.
    pub options_full: Vec<FieldOption>,
    /// Whether the field is mandatory.
    pub required: bool,
    /// Whether the field is part of the subscription form.
    pub in_form: bool,
    /// Whether the field is shown in the overview.
    pub in_list: bool,
    /// Whether this is the email field.
    pub is_email: bool,
    /// Autocomplete hint for forms.
    pub autocomplete: Option<String>,
}

/// The kind of field to create, including the options select fields need.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum NewFieldKind {
    /// Free text.
    Text,
    /// A number.
    Numeric,
    /// A date.
    Date,
    /// One choice out of `options`.
    SelectSingle {
        /// The available choices.
        options: Vec<String>,
        /// How the field is displayed in forms.
        display: Option<DisplayType>,
    },
    /// Multiple choices out of `options`.
    SelectMultiple {
        /// The available choices.
        options: Vec<String>,
    },
}

/// Parameters for creating a field.
#[derive(Debug, Clone)]
pub struct NewField {
    name: String,
    kind: NewFieldKind,
    defaultvalue: Option<String>,
    required: bool,
    in_form: bool,
    in_list: bool,
}

impl NewField {
    /// Starts a new field. It is optional, shown in forms and shown in the overview by default.
    pub fn new(name: impl Into<String>, kind: NewFieldKind) -> Self {
        Self {
            name: name.into(),
            kind,
            defaultvalue: None,
            required: false,
            in_form: true,
            in_list: true,
        }
    }

    /// Sets the value used when none is supplied.
    pub fn defaultvalue(mut self, value: impl Into<String>) -> Self {
        self.defaultvalue = Some(value.into());
        self
    }

    /// Makes the field mandatory.
    pub fn required(mut self, required: bool) -> Self {
        self.required = required;
        self
    }

    /// Sets whether the field is part of the subscription form.
    pub fn in_form(mut self, in_form: bool) -> Self {
        self.in_form = in_form;
        self
    }

    /// Sets whether the field is shown in the overview.
    pub fn in_list(mut self, in_list: bool) -> Self {
        self.in_list = in_list;
        self
    }

    fn body(&self, list_id: &ListId) -> serde_json::Value {
        let mut body = json!({
            "list_id": list_id,
            "name": self.name,
            "required": self.required,
            "in_form": self.in_form,
            "in_list": self.in_list,
        });
        let map = body.as_object_mut().expect("object literal");
        let datatype = match &self.kind {
            NewFieldKind::Text => "text",
            NewFieldKind::Numeric => "numeric",
            NewFieldKind::Date => "date",
            NewFieldKind::SelectSingle { options, display } => {
                map.insert("options".into(), json!(options));
                if let Some(display) = display {
                    map.insert("datatype_display".into(), json!(display));
                }
                "select_single"
            }
            NewFieldKind::SelectMultiple { options } => {
                map.insert("options".into(), json!(options));
                "select_multiple"
            }
        };
        map.insert("datatype".into(), json!(datatype));
        if let Some(default) = &self.defaultvalue {
            map.insert("defaultvalue".into(), json!(default));
        }
        body
    }
}

/// Parameters for updating a field. Only fields that are set are changed.
#[derive(Debug, Clone, Default, Serialize)]
pub struct UpdateField {
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    datatype: Option<WritableDataType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    datatype_display: Option<DisplayType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    options_full: Option<Vec<FieldOption>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    defaultvalue: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    required: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    in_form: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    in_list: Option<bool>,
}

impl UpdateField {
    /// Creates an update that changes nothing yet.
    pub fn new() -> Self {
        Self::default()
    }

    /// Renames the field.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Changes the data type. This removes all data stored in the field.
    pub fn datatype(mut self, datatype: WritableDataType) -> Self {
        self.datatype = Some(datatype);
        self
    }

    /// Changes how a single-choice field is displayed.
    pub fn datatype_display(mut self, display: DisplayType) -> Self {
        self.datatype_display = Some(display);
        self
    }

    /// Replaces the full set of options. Options that are left out are deleted.
    pub fn options_full(mut self, options: impl IntoIterator<Item = FieldOption>) -> Self {
        self.options_full = Some(options.into_iter().collect());
        self
    }

    /// Changes the default value.
    pub fn defaultvalue(mut self, value: impl Into<String>) -> Self {
        self.defaultvalue = Some(value.into());
        self
    }

    /// Changes whether the field is mandatory.
    pub fn required(mut self, required: bool) -> Self {
        self.required = Some(required);
        self
    }

    /// Changes whether the field is part of the subscription form.
    pub fn in_form(mut self, in_form: bool) -> Self {
        self.in_form = Some(in_form);
        self
    }

    /// Changes whether the field is shown in the overview.
    pub fn in_list(mut self, in_list: bool) -> Self {
        self.in_list = Some(in_list);
        self
    }
}

/// Operations on list fields, obtained from [`Client::fields`].
#[derive(Debug, Clone, Copy)]
pub struct Fields<'a> {
    client: &'a Client,
}

impl<'a> Fields<'a> {
    pub(crate) fn new(client: &'a Client) -> Self {
        Self { client }
    }

    /// Creates a field in a list.
    pub async fn create(&self, list_id: &ListId, field: &NewField) -> Result<Field> {
        let body = Body::Form(field.body(list_id));
        let response = self
            .client
            .send(Method::POST, &["field"], &Query::new(), body)
            .await?;
        decode(response, "field")
    }

    /// Fetches a field.
    pub async fn get(&self, list_id: &ListId, field_id: &FieldId) -> Result<Field> {
        let query = Query::new().with("list_id", list_id);
        let response = self
            .client
            .send(
                Method::GET,
                &["field", field_id.as_str()],
                &query,
                Body::None,
            )
            .await?;
        decode(response, "field")
    }

    /// Fetches all fields of a list.
    pub async fn all(&self, list_id: &ListId) -> Result<Vec<Field>> {
        let query = Query::new().with("list_id", list_id);
        let response = self
            .client
            .send(Method::GET, &["field"], &query, Body::None)
            .await?;
        decode_list(response, "field")
    }

    /// Updates a field.
    ///
    /// When `options_full` is set the request is sent as JSON, as the API requires.
    pub async fn update(
        &self,
        list_id: &ListId,
        field_id: &FieldId,
        update: &UpdateField,
    ) -> Result<Field> {
        let value = with_field(to_value(update)?, "list_id", list_id.as_str());
        let body = if update.options_full.is_some() {
            Body::Json(value)
        } else {
            Body::Form(value)
        };
        let response = self
            .client
            .send(
                Method::POST,
                &["field", field_id.as_str()],
                &Query::new(),
                body,
            )
            .await?;
        decode(response, "field")
    }

    /// Permanently deletes a field.
    pub async fn delete(&self, list_id: &ListId, field_id: &FieldId) -> Result<Field> {
        let query = Query::new().with("list_id", list_id);
        let response = self
            .client
            .send(
                Method::DELETE,
                &["field", field_id.as_str()],
                &query,
                Body::None,
            )
            .await?;
        decode(response, "field")
    }
}

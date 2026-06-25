//! Centralized logic symbol names used by solver terms.

/// Function symbols used to build structured data terms inside predicate arguments.
pub(in crate::logic) mod func {
    pub(in crate::logic) const ARG: &str = "#arg";
    pub(in crate::logic) const ARRAY: &str = "#array";
    pub(in crate::logic) const ASSOC_CONST_ARG: &str = "#assoc_const_arg";
    pub(in crate::logic) const ASSOC_TYPE_ARG: &str = "#assoc_type_arg";
    pub(in crate::logic) const CONCRETE: &str = "#concrete";
    pub(in crate::logic) const CONST_BOOL: &str = "#const_bool";
    pub(in crate::logic) const CONST_FLOAT: &str = "#const_float";
    pub(in crate::logic) const CONST_INT: &str = "#const_int";
    pub(in crate::logic) const DEF: &str = "#def";
    pub(in crate::logic) const GENERIC_PARAM: &str = "#generic_param";
    pub(in crate::logic) const IMPL_PATTERN: &str = "#impl_pattern";
    pub(in crate::logic) const INFER: &str = "#infer";
    pub(in crate::logic) const LEN_EXPR: &str = "#len_expr";
    pub(in crate::logic) const MUT: &str = "#mut";
    pub(in crate::logic) const NAME: &str = "#name";
    pub(in crate::logic) const PATH: &str = "#path";
    pub(in crate::logic) const PRIMITIVE: &str = "#primitive";
    pub(in crate::logic) const REF: &str = "#ref";
    pub(in crate::logic) const SLICE: &str = "#slice";
    pub(in crate::logic) const TUPLE: &str = "#tuple";
}

/// Predicate symbols used as logic database relations and queries.
pub(in crate::logic) mod pred {
    pub(in crate::logic) const EXPLICIT_PROJECTION_OBLIGATION: &str =
        "#explicit_projection_obligation";
    pub(in crate::logic) const IMPL_ASSOC_TYPE: &str = "#impl_assoc_type";
    pub(in crate::logic) const IMPL_SELF_MATCH: &str = "#impl_self_match";
    pub(in crate::logic) const IMPL_SELF_MATCH_CANDIDATE: &str = "#impl_self_match_candidate";
    pub(in crate::logic) const PROJECTION_CANDIDATE: &str = "#projection_candidate";
    pub(in crate::logic) const PROJECTION_MATCH: &str = "#projection_match";
    pub(in crate::logic) const PROJECTION_NORMALIZES_TO: &str = "#projection_normalizes_to";
    pub(in crate::logic) const PROJECTION_OBLIGATION: &str = "#projection_obligation";
    pub(in crate::logic) const RESOLVED_TYPE: &str = "#resolved_type";
    pub(in crate::logic) const SAME_TYPE: &str = "#same_type";
    pub(in crate::logic) const TRAIT_BOUND: &str = "#trait_bound";
    pub(in crate::logic) const TYPE_BINDING: &str = "#type_binding";
    pub(in crate::logic) const TYPE_CANDIDATE: &str = "#type_candidate";
    pub(in crate::logic) const TYPE_EQUAL: &str = "#type_equal";
    pub(in crate::logic) const TYPE_SHAPE: &str = "#type_shape";
    pub(in crate::logic) const TYPE_SUBSTITUTION: &str = "#type_substitution";
}

/// Variable symbols used in logic rules and open queries.
pub(in crate::logic) mod var {
    pub(in crate::logic) const ARG: &str = "$Arg";
    pub(in crate::logic) const ASSOC: &str = "$Assoc";
    pub(in crate::logic) const GENERIC: &str = "$Generic";
    pub(in crate::logic) const IMPL_SELF: &str = "$ImplSelf";
    pub(in crate::logic) const IMPL_TRAIT: &str = "$ImplTrait";
    pub(in crate::logic) const LEFT: &str = "$Left";
    pub(in crate::logic) const PROJECTION: &str = "$Projection";
    pub(in crate::logic) const RIGHT: &str = "$Right";
    pub(in crate::logic) const SELF: &str = "$Self";
    pub(in crate::logic) const SUBJECT: &str = "$Subject";
    pub(in crate::logic) const SUBSTITUTED: &str = "$Substituted";
    pub(in crate::logic) const TRAIT: &str = "$Trait";
    pub(in crate::logic) const TYPE: &str = "$Type";
    pub(in crate::logic) const VALUE: &str = "$Value";
    pub(in crate::logic) const SHAPE: &str = "$Shape";
}

use aegis_gateway::core::data_governance::PolicyTier;
use aegis_gateway::core::federation::{
    SamlServiceProvider, ScimEventType, ScimInboundReceiver, ScimUserRecord,
};
use aegis_gateway::policy::saml::NativeSamlServiceProvider;
use aegis_gateway::policy::scim::NativeScimInboundReceiver;

#[test]
fn test_saml_authn_request_and_valid_assertion_parsing() {
    let sp = NativeSamlServiceProvider::new(
        "https://gateway.aegis.enterprise.io/saml/metadata",
        "https://sts.windows.net/72f988bf-86f1-41af-91ab-2d7cd011db47/",
        "https://gateway.aegis.enterprise.io/saml/acs",
    );

    // 1. Generate AuthnRequest
    let req = sp
        .create_authn_request("https://login.microsoftonline.com/tenant/saml2")
        .expect("AuthnRequest creation failed");
    assert!(req.contains("samlp:AuthnRequest"));
    assert!(req.contains("https://gateway.aegis.enterprise.io/saml/acs"));
    assert!(req.contains("https://gateway.aegis.enterprise.io/saml/metadata"));

    // 2. Validate valid assertion XML
    let valid_xml = r#"
    <saml2p:Response xmlns:saml2p="urn:oasis:names:tc:SAML:2.0:protocol" ID="_resp123">
        <saml2:Assertion xmlns:saml2="urn:oasis:names:tc:SAML:2.0:assertion" ID="_assert456">
            <saml2:Issuer>https://sts.windows.net/72f988bf-86f1-41af-91ab-2d7cd011db47/</saml2:Issuer>
            <saml2:Conditions NotBefore="2026-10-10T00:00:00Z" NotOnOrAfter="2026-10-11T00:00:00Z">
                <saml2:AudienceRestriction>
                    <saml2:Audience>https://gateway.aegis.enterprise.io/saml/metadata</saml2:Audience>
                </saml2:AudienceRestriction>
            </saml2:Conditions>
            <saml2:Subject>
                <saml2:NameID>principal_engineer@enterprise.com</saml2:NameID>
            </saml2:Subject>
            <saml2:AttributeStatement>
                <saml2:Attribute Name="Department">
                    <saml2:AttributeValue>AI Infrastructure</saml2:AttributeValue>
                </saml2:Attribute>
                <saml2:Attribute Name="Role">
                    <saml2:AttributeValue>admin,auditor</saml2:AttributeValue>
                </saml2:Attribute>
            </saml2:AttributeStatement>
        </saml2:Assertion>
    </saml2p:Response>
    "#;

    let ctx = sp.validate_assertion(valid_xml).expect("Assertion validation failed");
    assert_eq!(ctx.name_id, "principal_engineer@enterprise.com");
    assert_eq!(ctx.issuer, "https://sts.windows.net/72f988bf-86f1-41af-91ab-2d7cd011db47/");
    assert_eq!(ctx.attributes.get("Department").unwrap(), "AI Infrastructure");
    assert_eq!(ctx.roles, vec!["admin".to_string(), "auditor".to_string()]);
}

#[test]
fn test_saml_security_refusals_and_mismatches() {
    let sp = NativeSamlServiceProvider::new(
        "https://gateway.aegis.enterprise.io/saml/metadata",
        "https://sts.windows.net/trusted-tenant/",
        "https://gateway.aegis.enterprise.io/saml/acs",
    );

    // Mismatched Issuer
    let bad_issuer_xml = r#"
    <samlp:Response xmlns:samlp="urn:oasis:names:tc:SAML:2.0:protocol">
        <saml:Assertion xmlns:saml="urn:oasis:names:tc:SAML:2.0:assertion">
            <saml:Issuer>https://rogue-idp.attacker.com/</saml:Issuer>
            <saml:Audience>https://gateway.aegis.enterprise.io/saml/metadata</saml:Audience>
        </saml:Assertion>
    </samlp:Response>
    "#;
    let res = sp.validate_assertion(bad_issuer_xml);
    assert!(res.is_err(), "Should reject mismatched issuer");

    // Mismatched Audience
    let bad_audience_xml = r#"
    <samlp:Response xmlns:samlp="urn:oasis:names:tc:SAML:2.0:protocol">
        <saml:Assertion xmlns:saml="urn:oasis:names:tc:SAML:2.0:assertion">
            <saml:Issuer>https://sts.windows.net/trusted-tenant/</saml:Issuer>
            <saml:Audience>https://another-gateway.com/metadata</saml:Audience>
        </saml:Assertion>
    </samlp:Response>
    "#;
    let res = sp.validate_assertion(bad_audience_xml);
    assert!(res.is_err(), "Should reject mismatched audience");

    // Expired Assertion
    let expired_xml = r#"
    <samlp:Response xmlns:samlp="urn:oasis:names:tc:SAML:2.0:protocol">
        <saml:Assertion xmlns:saml="urn:oasis:names:tc:SAML:2.0:assertion" NotOnOrAfter="2020-01-01T00:00:00Z">
            <saml:Issuer>https://sts.windows.net/trusted-tenant/</saml:Issuer>
            <saml:Audience>https://gateway.aegis.enterprise.io/saml/metadata</saml:Audience>
        </saml:Assertion>
    </samlp:Response>
    "#;
    let res = sp.validate_assertion(expired_xml);
    assert!(res.is_err(), "Should reject expired assertion");
}

#[test]
fn test_scim_lifecycle_provisioning_and_immediate_deprovisioning() {
    let scim = NativeScimInboundReceiver::new();

    // 1. Provision new user with Security-Admins group
    let user_record = ScimUserRecord {
        user_id: "usr_corp_99182".to_string(),
        username: "sarah.connor".to_string(),
        email: "sarah.connor@enterprise.io".to_string(),
        active: true,
        groups: vec!["Security-Admins".to_string(), "Developers".to_string()],
    };

    let prov_res = scim
        .process_user_event(ScimEventType::CreateUser, user_record.clone())
        .expect("SCIM user creation failed");
    assert!(prov_res.active_status);
    assert_eq!(prov_res.mapped_tier, PolicyTier::StrictAirgapped);
    assert!(!scim.is_user_revoked("usr_corp_99182"));

    // 2. Deprovision user immediately (employee termination event)
    let deprov_res = scim
        .process_user_event(ScimEventType::DeprovisionUser, user_record)
        .expect("SCIM deprovision failed");
    assert!(!deprov_res.active_status);
    assert!(scim.is_user_revoked("usr_corp_99182"));
    assert!(deprov_res.message.contains("tombstoned"));
}

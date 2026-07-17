use rcgen::{
    BasicConstraints, CertificateParams, CertifiedIssuer, DnType, ExtendedKeyUsagePurpose, IsCa,
    KeyPair, KeyUsagePurpose,
};

pub fn certificate_authority(name: &str) -> CertifiedIssuer<'static, KeyPair> {
    let mut parameters = CertificateParams::new(Vec::new()).unwrap();
    parameters.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    parameters.distinguished_name.push(DnType::CommonName, name);
    parameters.key_usages = vec![
        KeyUsagePurpose::DigitalSignature,
        KeyUsagePurpose::KeyCertSign,
        KeyUsagePurpose::CrlSign,
    ];
    CertifiedIssuer::self_signed(parameters, KeyPair::generate().unwrap()).unwrap()
}

pub fn leaf_certificate(
    name: &str,
    usage: ExtendedKeyUsagePurpose,
    issuer: &CertifiedIssuer<'_, KeyPair>,
) -> (String, String) {
    let mut parameters = CertificateParams::new(vec![name.to_owned()]).unwrap();
    parameters.distinguished_name.push(DnType::CommonName, name);
    parameters.key_usages = vec![KeyUsagePurpose::DigitalSignature];
    parameters.extended_key_usages = vec![usage];
    let key = KeyPair::generate().unwrap();
    let certificate = parameters.signed_by(&key, issuer).unwrap();
    (certificate.pem(), key.serialize_pem())
}

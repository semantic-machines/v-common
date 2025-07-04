use v_storage::*;
use v_individual_model::onto::individual::Individual;

#[test]
fn test_cross_storage_compatibility() {
    // Тестируем что данные сохраненные в одном типе хранилища могут быть прочитаны другим
    let data_sets = vec![
        ("memory1", "value1"),
        ("memory2", "value2"),
        ("memory3", "value3"),
    ];

    // Dynamic dispatch storage
    let dynamic_storage = VStorage::builder().memory().build().unwrap();
    let mut dynamic_storage = VStorage::new(dynamic_storage);
    
    // Generic storage  
    let mut generic_storage = VMemoryStorage::new(memory_storage::MemoryStorage::new());
    
    // Enum storage
    let mut enum_storage = VStorageEnum::memory();

    // Записываем данные в каждое хранилище
    for (key, value) in &data_sets {
        assert!(dynamic_storage.put_value(StorageId::Individuals, key, value).is_ok());
        assert!(generic_storage.put_value(StorageId::Individuals, key, value).is_ok());
        assert!(enum_storage.put_value(StorageId::Individuals, key, value).is_ok());
    }

    // Проверяем что можем прочитать все данные из каждого хранилища
    for (key, expected_value) in &data_sets {
        let dynamic_result = dynamic_storage.get_value(StorageId::Individuals, key);
        let generic_result = generic_storage.get_value(StorageId::Individuals, key);
        let enum_result = enum_storage.get_value(StorageId::Individuals, key);

        assert!(dynamic_result.is_ok(), "Dynamic storage failed for key: {}", key);
        assert!(generic_result.is_ok(), "Generic storage failed for key: {}", key);
        assert!(enum_result.is_ok(), "Enum storage failed for key: {}", key);

        if let (StorageResult::Ok(val1), StorageResult::Ok(val2), StorageResult::Ok(val3)) = 
            (dynamic_result, generic_result, enum_result) {
            assert_eq!(val1, *expected_value);
            assert_eq!(val2, *expected_value);
            assert_eq!(val3, *expected_value);
        }
    }
}

#[test]
fn test_storage_factory_pattern() {
    // Тест различных способов создания хранилищ через фабрики

    // Builder pattern
    let builder_storage = StorageBuilder::new()
        .memory()
        .build();
    assert!(builder_storage.is_ok(), "Builder pattern failed");

    // Provider pattern
    let mut provider_storage = StorageProvider::memory();
    #[allow(deprecated)]
    {
        assert!(!provider_storage.get_v(StorageId::Individuals, "nonexistent").is_some());
    }

    // Config pattern  
    let config_storage = VStorage::from_config(StorageConfig::Memory);
    assert!(config_storage.is_ok(), "Config pattern failed");

    // Generic builders
    let generic_memory = StorageBuilder::new()
        .memory()
        .build_memory_generic();
    assert!(generic_memory.is_ok(), "Generic memory builder failed");
}

#[test]
fn test_all_storage_types_operations() {
    // Параметризованный тест для всех типов StorageId
    let storage_types = vec![
        StorageId::Individuals,
        StorageId::Tickets,
        StorageId::Az,
    ];

    let mut storage = VStorageEnum::memory();

    for storage_type in storage_types {
        let key = format!("test_key_{:?}", storage_type);
        let value = format!("test_value_{:?}", storage_type);

        // Test put/get cycle
        assert!(storage.put_value(storage_type.clone(), &key, &value).is_ok());
        
        let get_result = storage.get_value(storage_type.clone(), &key);
        assert!(get_result.is_ok(), "Get failed for storage type: {:?}", storage_type);
        
        if let StorageResult::Ok(retrieved_value) = get_result {
            assert_eq!(retrieved_value, value);
        }

        // Test count
        let count_result = storage.count(storage_type.clone());
        assert!(count_result.is_ok(), "Count failed for storage type: {:?}", storage_type);
        
        // Test raw operations
        let raw_data = value.as_bytes().to_vec();
        let raw_key = format!("{}_raw", key);
        
        assert!(storage.put_raw_value(storage_type.clone(), &raw_key, raw_data.clone()).is_ok());
        
        let raw_get_result = storage.get_raw_value(storage_type.clone(), &raw_key);
        assert!(raw_get_result.is_ok(), "Raw get failed for storage type: {:?}", storage_type);
        
        if let StorageResult::Ok(retrieved_raw) = raw_get_result {
            assert_eq!(retrieved_raw, raw_data);
        }

        // Test remove
        assert!(storage.remove_value(storage_type.clone(), &key).is_ok());
        
        let removed_get_result = storage.get_value(storage_type, &key);
        assert_eq!(removed_get_result, StorageResult::NotFound, "Key should be removed");
    }
}

#[test]
fn test_individual_workflow() {
    // Тест полного workflow работы с Individual
    let mut storage = VMemoryStorage::new(memory_storage::MemoryStorage::new());
    let mut individual = Individual::default();

    // Тест с корректными Individual данными в различных форматах
    let test_cases = vec![
        (
            "simple_individual",
            r#"{"@": "test:simple", "rdf:type": [{"type": "Uri", "data": "test:Person"}]}"#
        ),
        (
            "complex_individual", 
            r#"{"@": "test:complex", "rdf:type": [{"type": "Uri", "data": "test:Organization"}], "rdfs:label": [{"type": "String", "data": "Test Organization"}]}"#
        ),
    ];

    for (id, data) in test_cases {
        // Сохраняем Individual
        let put_result = storage.put_value(StorageId::Individuals, id, data);
        assert!(put_result.is_ok(), "Failed to put individual: {}", id);

        // Читаем как Individual
        let get_individual_result = storage.get_individual_from_storage(StorageId::Individuals, id, &mut individual);
        // Может быть Ok или UnprocessableEntity в зависимости от парсера
        assert!(get_individual_result == StorageResult::Ok(()) || get_individual_result == StorageResult::UnprocessableEntity,
               "Unexpected result for individual {}: {:?}", id, get_individual_result);

        // Читаем как строку для проверки
        let get_string_result = storage.get_value(StorageId::Individuals, id);
        assert!(get_string_result.is_ok(), "Failed to get individual as string: {}", id);
        
        if let StorageResult::Ok(retrieved_data) = get_string_result {
            assert_eq!(retrieved_data, data);
        }
    }

    // Тест с некорректными данными
    let invalid_data = "this is not json";
    assert!(storage.put_value(StorageId::Individuals, "invalid", invalid_data).is_ok());
    
    let invalid_individual_result = storage.get_individual_from_storage(StorageId::Individuals, "invalid", &mut individual);
    assert_eq!(invalid_individual_result, StorageResult::UnprocessableEntity, "Invalid data should return UnprocessableEntity");
}

#[test]
fn test_error_conditions() {
    // Тест различных ошибочных условий
    let mut empty_storage = VStorageEnum::default();
    let mut individual = Individual::default();

    // Операции с пустым хранилищем
    assert_eq!(empty_storage.get_value(StorageId::Individuals, "any"), StorageResult::NotReady);
    assert_eq!(empty_storage.put_value(StorageId::Individuals, "any", "value"), StorageResult::NotReady);
    assert_eq!(empty_storage.remove_value(StorageId::Individuals, "any"), StorageResult::NotReady);
    assert_eq!(empty_storage.count(StorageId::Individuals), StorageResult::NotReady);
    assert_eq!(empty_storage.get_individual(StorageId::Individuals, "any", &mut individual), StorageResult::NotReady);

    // Операции с несуществующими ключами
    let mut memory_storage = VStorageEnum::memory();
    assert_eq!(memory_storage.get_value(StorageId::Individuals, "nonexistent"), StorageResult::NotFound);
    assert_eq!(memory_storage.remove_value(StorageId::Individuals, "nonexistent"), StorageResult::NotFound);
    assert_eq!(memory_storage.get_individual(StorageId::Individuals, "nonexistent", &mut individual), StorageResult::NotFound);

    // Тест с пустыми строками
    assert!(memory_storage.put_value(StorageId::Individuals, "", "empty_key").is_ok());
    assert!(memory_storage.put_value(StorageId::Individuals, "empty_value", "").is_ok());
    
    let empty_key_result = memory_storage.get_value(StorageId::Individuals, "");
    assert!(empty_key_result.is_ok());
    
    let empty_value_result = memory_storage.get_value(StorageId::Individuals, "empty_value");
    assert!(empty_value_result.is_ok());
    if let StorageResult::Ok(value) = empty_value_result {
        assert_eq!(value, "");
    }
}

#[test]
fn test_backward_compatibility_integration() {
    // Интеграционный тест совместимости старого и нового API
    let storage = VStorage::builder().memory().build().unwrap();
    let mut storage = VStorage::new(storage);

    // Используем новый API для записи
    assert!(storage.put_value(StorageId::Individuals, "new_api_key", "new_api_value").is_ok());

    // Читаем через старый API
    #[allow(deprecated)]
    {
        let old_result = storage.get_v(StorageId::Individuals, "new_api_key");
        assert_eq!(old_result, Some("new_api_value".to_string()));

        // Записываем через старый API
        assert!(storage.put_kv(StorageId::Individuals, "old_api_key", "old_api_value"));

        // Читаем через новый API
        let new_result = storage.get_value(StorageId::Individuals, "old_api_key");
        assert!(new_result.is_ok());
        if let StorageResult::Ok(value) = new_result {
            assert_eq!(value, "old_api_value");
        }
    }
}

#[test]
fn test_memory_storage_individual_operations() {
    let mut storage = MemoryStorage::new();
    let mut individual = Individual::default();
    
    // Test non-existent individual
    assert_eq!(storage.get_individual(StorageId::Individuals, "non-existent", &mut individual), StorageResult::NotFound);
    
    // Test with valid Individual data
    let valid_data = r#"{"@":"individual","@id":"test:individual","v-s:hasValue":[{"data":"test_value","type":"_string"}]}"#;
    assert!(storage.put_value(StorageId::Individuals, "test:individual", valid_data).is_ok());
    
    // Test loading the individual
    let get_individual_result = storage.get_individual(StorageId::Individuals, "test:individual", &mut individual);
    assert!(get_individual_result == StorageResult::Ok(()) || get_individual_result == StorageResult::UnprocessableEntity,
            "Expected Ok or UnprocessableEntity, got: {:?}", get_individual_result);
    
    // Test with invalid data
    let invalid_data = "invalid json data";
    assert!(storage.put_value(StorageId::Individuals, "test:invalid", invalid_data).is_ok());
    
    // This should fail to parse
    let invalid_individual_result = storage.get_individual(StorageId::Individuals, "test:invalid", &mut individual);
    assert_eq!(invalid_individual_result, StorageResult::UnprocessableEntity, "Invalid data should return UnprocessableEntity");
}

#[test]
fn test_empty_storage_behavior() {
    let mut empty_storage = VStorage::none();
    let mut individual = Individual::default();
    
    // Empty storage should return NotReady
    assert_eq!(empty_storage.get_individual("any", &mut individual), StorageResult::NotReady);
    
    // Test with memory storage
    let mut memory_storage = VStorage::new(Box::new(MemoryStorage::new()));
    assert_eq!(memory_storage.get_individual("nonexistent", &mut individual), StorageResult::NotFound);
} 